use futures::future::join_all;
use octocrab::models::repos;
use octocrab::repos::RepoHandler;
use octocrab::{Octocrab, OctocrabBuilder};
use semver::Version;

use crate::config::ApiConfig;
use crate::errors::{InternalError, Result};
use crate::game_data::{Asset, Assets, GameRelease, Repo};

pub struct Fetcher {
    octocrab: Octocrab,
    game_repo: Repo,
    updater_repo: Repo,

    checksum_fetcher: ChecksumFetcher,
}

struct ChecksumFetcher(reqwest::Client);

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

    pub async fn get_latest_game_release(&self) -> Result<GameRelease> {
        let releases = self
            .on_repo(&self.game_repo)
            .releases()
            .list()
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
            .map(|((platform, mut asset), sha256)| {
                asset.sha256 = sha256?;
                Ok((platform.to_string(), asset))
            })
            .collect::<Result<Assets>>()?;

        for (version, release) in versions_released {
            for ((platform, mut asset), sha256) in self
                .get_assets_and_checksums(&release.assets, &version, Some(&binaries))
                .await
            {
                asset.sha256 = sha256?;
                binaries.insert(platform.to_string(), asset);
            }
        }

        let latest_assets = binaries.remove("assets");

        match latest_assets {
            Some(assets) => Ok(GameRelease {
                assets_version: assets.version.clone(),
                assets,
                binaries,
                version: latest_version,
            }),
            None => Err(InternalError::NoReleaseFound),
        }
    }

    pub async fn get_latest_updater_release(&self) -> Result<Assets> {
        let last_release = self
            .on_repo(&self.updater_repo)
            .releases()
            .get_latest()
            .await?;

        let version = Version::parse(&last_release.tag_name)?;

        self.get_assets_and_checksums(&last_release.assets, &version, None)
            .await
            .map(|((platform, mut asset), sha256)| {
                asset.sha256 = sha256?;
                Ok((platform.to_string(), asset))
            })
            .collect::<Result<Assets>>()
    }

    async fn get_assets_and_checksums<'a: 'b, 'b, A>(
        &self,
        assets: A,
        version: &Version,
        binaries: Option<&Assets>,
    ) -> impl Iterator<Item = ((&'b str, Asset), Result<Option<String>>)>
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
        Self(reqwest::Client::new())
    }

    async fn resolve(&self, asset: &Asset) -> Result<Option<String>> {
        // Try to get the SHA256 file
        let response = self
            .0
            .get(format!("{}.sha256", asset.download_url))
            .send()
            .await?;

        match response.status().is_client_error() {
            // No SHA256 found
            true => Ok(None),
            false => {
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
