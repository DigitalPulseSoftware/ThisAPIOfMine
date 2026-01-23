use std::time::Duration;

use futures::future::join_all;
use octocrab::models::repos;
use octocrab::repos::RepoHandler;
use octocrab::{Octocrab, OctocrabBuilder};
use reqwest::StatusCode;
use semver::Version;

use crate::config::ApiConfig;
use crate::errors::{InternalError, Result};
use crate::game_data::{Asset, AssetList, AssetPerPlatform, GameReleases, Repo};

pub struct Fetcher {
    octocrab: Octocrab,
    game_repo: Repo,
    updater_repo: Repo,

    checksum_fetcher: ChecksumFetcher,
}

struct ChecksumFetcher(reqwest_middleware::ClientWithMiddleware);

impl Fetcher {
    pub fn from_config(config: &ApiConfig) -> Result<Self> {
        let mut octocrab = OctocrabBuilder::default();
        if let Some(github_pat) = &config.github_pat {
            octocrab = octocrab.personal_token(github_pat.unsecure().to_string());
        }

        Ok(Self {
            octocrab: octocrab.build()?,
            game_repo: Repo::new(&config.repo_owner, &config.game_repository),
            updater_repo: Repo::new(&config.repo_owner, &config.updater_repository),

            checksum_fetcher: ChecksumFetcher::new(),
        })
    }

    fn on_repo(&self, repo: &Repo) -> RepoHandler<'_> {
        self.octocrab.repos(repo.owner(), repo.repository())
    }

    pub async fn get_latest_game_releases(&self) -> Result<GameReleases> {
        let releases = self
            .on_repo(&self.game_repo)
            .releases()
            .list()
            .per_page(100)
            .send()
            .await?;

        let mut versions_released = releases
            .into_iter()
            .filter(|r| !r.prerelease)
            .filter_map(|r| Version::parse(&r.tag_name).ok().map(|v| (v, r)));

        let Some((latest_version, latest_release)) = versions_released.next() else {
            return Err(InternalError::NoReleaseFound);
        };

        let mut binaries = self
            .get_assets_and_checksums(&latest_release.assets, &latest_version, None)
            .await
            .filter_map(|((platform, mut asset), sha256)| {
                match sha256 {
                    Ok(checksum) => {
                        asset.sha256 = checksum;
                        Some(Ok((platform.to_string(), asset)))
                    },
                    Err(err) => {
                        log::error!("ignoring asset {0} (version: {1}) because an error occurred for checksum: {2:?}", asset.name, asset.version, err);
                        None
                    }
                }
            })
            .collect::<Result<AssetPerPlatform>>()?;

        let mut assets = AssetList::new();

        for (version, release) in versions_released {
            for ((platform, mut asset), sha256) in self
                .get_assets_and_checksums(&release.assets, &version, Some(&binaries))
                .await
            {
                match sha256 {
                    Ok(checksum) => {
                        asset.sha256 = checksum;

                        if platform == "assets" {
                            assets.push(asset);
                        } else {
                            binaries.insert(platform.to_string(), asset);
                        }
                    }
                    Err(err) => {
                        log::error!(
                            "ignoring asset {0} (version: {1}) because an error occurred for checksum: {2:?}",
                            asset.name,
                            asset.version,
                            err
                        );
                    }
                }
            }
        }

        if binaries.is_empty() {
            return Err(InternalError::NoReleaseFound);
        }

        Ok(GameReleases { assets, binaries })
    }

    pub async fn get_latest_updater_release(&self) -> Result<AssetPerPlatform> {
        let last_release = self
            .on_repo(&self.updater_repo)
            .releases()
            .get_latest()
            .await?;

        let version = Version::parse(&last_release.tag_name)?;

        self.get_assets_and_checksums(&last_release.assets, &version, None)
            .await
            .filter_map(|((platform, mut asset), sha256)| {
                match sha256 {
                    Ok(checksum) => {
                        asset.sha256 = checksum;
                        Some(Ok((platform.to_string(), asset)))
                    },
                    Err(err) => {
                        log::error!("ignoring updater {0} (version: {1}) because an error occurred for checksum: {2:?}", asset.name, asset.version, err);
                        None
                    }
                }
            })
            .collect::<Result<AssetPerPlatform>>()
    }

    async fn get_assets_and_checksums<'a: 'b, 'b, A>(
        &self,
        assets: A,
        version: &Version,
        binaries: Option<&AssetPerPlatform>,
    ) -> impl Iterator<Item = ((&'b str, Asset), Result<Option<String>>)> + use<'b, A>
    where
        A: IntoIterator<Item = &'a repos::Asset>,
    {
        let assets = assets
            .into_iter()
            .filter_map(|asset| {
                let platform = remove_game_suffix(asset.name.as_str());
                match !asset.name.ends_with(".sha256")
                    && !binaries.is_some_and(|b| b.contains_key(platform))
                {
                    true => Some((platform, Asset::with_version(asset, version.clone()))),
                    false => None,
                }
            })
            .collect::<Vec<(&str, Asset)>>();

        let checksums = join_all(
            assets
                .iter()
                .map(|(_, asset)| self.checksum_fetcher.resolve(asset)),
        )
        .await;

        assets.into_iter().zip(checksums)
    }
}

impl ChecksumFetcher {
    fn new() -> Self {
        let retry_policy = reqwest_retry::policies::ExponentialBackoff::builder()
            .build_with_total_retry_duration_and_max_retries(Duration::from_secs(15), 3);

        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
            .with(reqwest_retry::RetryTransientMiddleware::new_with_policy(
                retry_policy,
            ))
            .build();

        Self(client)
    }

    async fn resolve(&self, asset: &Asset) -> Result<Option<String>> {
        // Try to get the SHA256 file
        let response = self
            .0
            .get(format!("{}.sha256", asset.download_url))
            .send()
            .await?;

        match response.status() {
            StatusCode::NOT_FOUND => Ok(None),
            _ => {
                let content = response.text().await?;
                self.parse_response(asset.name.as_str(), content.as_str())
                    .map(Some)
            }
        }
    }

    fn parse_response(&self, asset_name: &str, response: &str) -> Result<String> {
        let parts: Vec<_> = response.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(InternalError::InvalidSha256(
                parts.len(),
                asset_name.to_string(),
            ));
        }

        let (sha256, filename) = (parts[0], parts[1]);
        match !filename.starts_with('*') || &filename[1..] != asset_name {
            false => Ok(sha256.to_string()),
            true => Err(InternalError::WrongChecksum(asset_name.to_string())),
        }
    }
}

fn remove_game_suffix(asset_name: &str) -> &str {
    let platform = asset_name
        .find('.')
        .map_or(asset_name, |pos| &asset_name[..pos]);
    platform
        .find("_releasedbg")
        .map_or(platform, |pos| &platform[..pos])
}
