use std::collections::HashMap;

use octocrab::models::repos;
use semver::Version;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Asset {
    pub size: i64,
    // serialisation skipped to race with the previous api
    #[serde(skip_serializing)]
    pub name: String,
    #[serde(skip_serializing)]
    pub version: Version,
    pub download_url: String,
    pub sha256: Option<String>,
}

pub struct Repo {
    owner: String,
    repository: String,
}

pub type AssetList = Vec<Asset>;
pub type AssetPerPlatform = HashMap<String /*platform*/, Asset>;

#[derive(Clone)]
pub struct GameReleases {
    pub assets: AssetList,
    pub binaries: AssetPerPlatform,
}

#[derive(Serialize)]
pub struct GameVersion {
    pub assets: Asset,
    pub assets_version: String,
    pub binaries: Asset,
    pub updater: Asset,
    pub version: String,
}

impl Asset {
    pub fn with_version(asset: &repos::Asset, version: Version) -> Self {
        Self {
            size: asset.size,
            name: asset.name.clone(),
            download_url: asset.browser_download_url.to_string(),
            sha256: None,
            version,
        }
    }
}

impl Repo {
    pub fn new<O: ToString, R: ToString>(owner: O, repository: R) -> Self {
        Self {
            owner: owner.to_string(),
            repository: repository.to_string(),
        }
    }

    pub fn owner(&self) -> &str {
        self.owner.as_str()
    }

    pub fn repository(&self) -> &str {
        self.repository.as_str()
    }
}
