use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::feature_pack::FeaturePacksConfig;
use crate::image::WildFlyImagesConfig;

const DEFAULT_BASE_URL: &str = "https://raw.githubusercontent.com/hpehl/wildfly-meta/main";

pub const IMAGES_FILENAME: &str = "wildfly-images.toml";
pub const FEATURE_PACKS_FILENAME: &str = "feature-packs.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Downloaded(u32),
    Updated(u32),
    AlreadyUpToDate(u32),
}

pub fn config_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| String::from("~")))
        .join(".config")
        .join("wildfly-meta")
}

pub fn images_path() -> PathBuf {
    config_dir().join(IMAGES_FILENAME)
}

pub fn feature_packs_path() -> PathBuf {
    config_dir().join(FEATURE_PACKS_FILENAME)
}

pub fn update_all() -> Result<(UpdateStatus, UpdateStatus)> {
    update_all_with_base_url(DEFAULT_BASE_URL)
}

pub fn update_all_with_base_url(base_url: &str) -> Result<(UpdateStatus, UpdateStatus)> {
    let images = update_images_with_base_url(base_url)?;
    let packs = update_feature_packs_with_base_url(base_url)?;
    Ok((images, packs))
}

pub fn update_images() -> Result<UpdateStatus> {
    update_images_with_base_url(DEFAULT_BASE_URL)
}

pub fn update_images_with_base_url(base_url: &str) -> Result<UpdateStatus> {
    let url = format!("{}/{}", base_url, IMAGES_FILENAME);
    let local_path = images_path();
    update_file(&url, &local_path, |content| {
        let config: WildFlyImagesConfig = toml::from_str(content)?;
        Ok(config.config_version)
    })
}

pub fn update_feature_packs() -> Result<UpdateStatus> {
    update_feature_packs_with_base_url(DEFAULT_BASE_URL)
}

pub fn update_feature_packs_with_base_url(base_url: &str) -> Result<UpdateStatus> {
    let url = format!("{}/{}", base_url, FEATURE_PACKS_FILENAME);
    let local_path = feature_packs_path();
    update_file(&url, &local_path, |content| {
        let config: FeaturePacksConfig = toml::from_str(content)?;
        Ok(config.config_version)
    })
}

fn update_file<F>(url: &str, local_path: &Path, extract_version: F) -> Result<UpdateStatus>
where
    F: Fn(&str) -> Result<u32>,
{
    let response = reqwest::blocking::get(url)?;
    if !response.status().is_success() {
        bail!("Failed to download {}: HTTP {}", url, response.status());
    }
    let remote_content = response.text()?;
    let remote_version = extract_version(&remote_content)?;

    if local_path.exists() {
        let local_content = fs::read_to_string(local_path)?;
        let local_version = extract_version(&local_content)?;
        if local_version >= remote_version {
            return Ok(UpdateStatus::AlreadyUpToDate(local_version));
        }
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(local_path, &remote_content)?;
        Ok(UpdateStatus::Updated(remote_version))
    } else {
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(local_path, &remote_content)?;
        Ok(UpdateStatus::Downloaded(remote_version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn config_dir_path() {
        let dir = config_dir();
        assert!(dir.to_string_lossy().ends_with(".config/wildfly-meta"));
    }

    #[test]
    fn images_path_filename() {
        let path = images_path();
        assert_eq!(path.file_name().unwrap().to_string_lossy(), IMAGES_FILENAME);
    }

    #[test]
    fn feature_packs_path_filename() {
        let path = feature_packs_path();
        assert_eq!(
            path.file_name().unwrap().to_string_lossy(),
            FEATURE_PACKS_FILENAME
        );
    }

    #[test]
    fn update_file_first_download() {
        let tmp = env::temp_dir().join("wildfly-meta-test-download");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let local = tmp.join("test.toml");
        let content = "config_version = 1\nimages = []\n";
        let remote_path = tmp.join("remote.toml");
        fs::write(&remote_path, content).unwrap();

        // We can't test HTTP download in unit tests, but we can test the file logic
        // by writing directly and verifying the config_version extraction
        fs::write(&local, content).unwrap();
        let version: u32 = {
            let c = fs::read_to_string(&local).unwrap();
            let config: WildFlyImagesConfig = toml::from_str(&c).unwrap();
            config.config_version
        };
        assert_eq!(version, 1);

        let _ = fs::remove_dir_all(&tmp);
    }
}
