//! On-demand download and update of TOML configuration files from GitHub.
//!
//! Configuration files are stored in `~/.config/wildfly-meta/`. The update functions compare
//! the local `config_version` against the remote version and only re-download when the remote
//! is newer.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::feature_pack::{FeaturePackRegistry, FeaturePacksConfig};
use crate::wildfly_image::{WildFlyImageRegistry, WildFlyImagesConfig};

const DEFAULT_BASE_URL: &str = "https://raw.githubusercontent.com/hpehl/wildfly-meta/main";

/// Filename for the WildFly images TOML configuration.
pub const WILDFLY_IMAGES_FILENAME: &str = "wildfly-images.toml";

/// Filename for the feature packs TOML configuration.
pub const FEATURE_PACKS_FILENAME: &str = "feature-packs.toml";

/// Describes the entries added and removed between two configuration versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDiff {
    /// Display names of newly added entries.
    pub added: Vec<String>,
    /// Display names of removed entries.
    pub removed: Vec<String>,
}

/// Outcome of an update operation for a single configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// The file was downloaded for the first time.
    Downloaded {
        /// The config version of the downloaded file.
        version: u32,
        /// Number of entries in the downloaded file.
        count: usize,
    },
    /// The local file was updated to a newer remote version.
    Updated {
        /// The config version before the update.
        from_version: u32,
        /// The config version after the update.
        to_version: u32,
        /// Entries added and removed between the two versions.
        diff: UpdateDiff,
    },
    /// The local file is already at or ahead of the remote version.
    AlreadyUpToDate(u32),
}

impl UpdateStatus {
    /// Returns a human-readable summary line for the given label (e.g. `"WildFly images"`).
    pub fn summary(&self, label: &str) -> String {
        match self {
            UpdateStatus::Downloaded { version, count } => {
                format!("{label} downloaded ({count} entries, version {version})")
            }
            UpdateStatus::Updated {
                from_version,
                to_version,
                diff,
            } => {
                let mut s = format!("{label} updated from version {from_version} to {to_version}");
                if !diff.added.is_empty() {
                    s.push_str(&format!("\n  Added: {}", diff.added.join(", ")));
                }
                if !diff.removed.is_empty() {
                    s.push_str(&format!("\n  Removed: {}", diff.removed.join(", ")));
                }
                s
            }
            UpdateStatus::AlreadyUpToDate(version) => {
                format!("{label} already up to date (version {version})")
            }
        }
    }
}

/// Combined result of updating both WildFly images and feature packs configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateResult {
    /// Status of the WildFly images update.
    pub wildfly_images: UpdateStatus,
    /// Status of the feature packs update.
    pub feature_packs: UpdateStatus,
}

impl UpdateResult {
    /// Returns a human-readable summary of both update statuses.
    pub fn summary(&self) -> String {
        let wildfly_images = self.wildfly_images.summary("WildFly images");
        let feature_packs = self.feature_packs.summary("Feature packs");
        format!("{wildfly_images}\n{feature_packs}")
    }
}

/// Returns the configuration directory path (`~/.config/wildfly-meta/`).
pub fn config_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| String::from("~")))
        .join(".config")
        .join("wildfly-meta")
}

/// Returns the full path to the local WildFly images TOML file.
pub fn wildfly_images_path() -> PathBuf {
    config_dir().join(WILDFLY_IMAGES_FILENAME)
}

/// Returns the full path to the local feature packs TOML file.
pub fn feature_packs_path() -> PathBuf {
    config_dir().join(FEATURE_PACKS_FILENAME)
}

/// Downloads or updates both configuration files from the default GitHub URL.
pub fn update_all() -> Result<UpdateResult> {
    update_all_with_base_url(DEFAULT_BASE_URL)
}

/// Downloads or updates both configuration files from a custom base URL.
pub fn update_all_with_base_url(base_url: &str) -> Result<UpdateResult> {
    let wildfly_images = update_wildfly_images_with_base_url(base_url)?;
    let feature_packs = update_feature_packs_with_base_url(base_url)?;
    Ok(UpdateResult {
        wildfly_images,
        feature_packs,
    })
}

/// Downloads or updates the WildFly images configuration from the default GitHub URL.
pub fn update_wildfly_images() -> Result<UpdateStatus> {
    update_wildfly_images_with_base_url(DEFAULT_BASE_URL)
}

/// Downloads or updates the WildFly images configuration from a custom base URL.
pub fn update_wildfly_images_with_base_url(base_url: &str) -> Result<UpdateStatus> {
    let url = format!("{}/{}", base_url, WILDFLY_IMAGES_FILENAME);
    let local_path = wildfly_images_path();
    update_file(
        &url,
        &local_path,
        |content| {
            let config: WildFlyImagesConfig = toml::from_str(content)?;
            Ok((config.config_version, config.wildfly_images.len()))
        },
        |old_content, new_content| {
            compute_registry_diff(
                old_content,
                new_content,
                WildFlyImageRegistry::from_toml,
                |reg| reg.keys().copied().collect(),
                |reg, k| reg.get(*k).map(|wi| wi.full_name()),
            )
        },
    )
}

/// Downloads or updates the feature packs configuration from the default GitHub URL.
pub fn update_feature_packs() -> Result<UpdateStatus> {
    update_feature_packs_with_base_url(DEFAULT_BASE_URL)
}

/// Downloads or updates the feature packs configuration from a custom base URL.
pub fn update_feature_packs_with_base_url(base_url: &str) -> Result<UpdateStatus> {
    let url = format!("{}/{}", base_url, FEATURE_PACKS_FILENAME);
    let local_path = feature_packs_path();
    update_file(
        &url,
        &local_path,
        |content| {
            let config: FeaturePacksConfig = toml::from_str(content)?;
            Ok((config.config_version, config.feature_packs.len()))
        },
        |old_content, new_content| {
            compute_registry_diff(
                old_content,
                new_content,
                FeaturePackRegistry::from_toml,
                |reg| reg.keys().cloned().collect(),
                |reg, (s, v)| reg.get(s, &v.to_string()).map(|fp| fp.short_name()),
            )
        },
    )
}

fn compute_registry_diff<R, K: Ord>(
    old_content: &str,
    new_content: &str,
    parse: impl Fn(&str) -> Result<R>,
    keys: impl Fn(&R) -> BTreeSet<K>,
    lookup: impl Fn(&R, &K) -> Option<String>,
) -> UpdateDiff {
    match (parse(old_content), parse(new_content)) {
        (Ok(old_reg), Ok(new_reg)) => {
            let old_keys = keys(&old_reg);
            let new_keys = keys(&new_reg);
            let added = new_keys
                .difference(&old_keys)
                .filter_map(|k| lookup(&new_reg, k))
                .collect();
            let removed = old_keys
                .difference(&new_keys)
                .filter_map(|k| lookup(&old_reg, k))
                .collect();
            UpdateDiff { added, removed }
        }
        _ => UpdateDiff {
            added: vec![],
            removed: vec![],
        },
    }
}

fn update_file<F, D>(
    url: &str,
    local_path: &Path,
    extract_version: F,
    compute_diff: D,
) -> Result<UpdateStatus>
where
    F: Fn(&str) -> Result<(u32, usize)>,
    D: Fn(&str, &str) -> UpdateDiff,
{
    let response = reqwest::blocking::get(url)?;
    if !response.status().is_success() {
        bail!("Failed to download {}: HTTP {}", url, response.status());
    }
    let remote_content = response.text()?;
    let (remote_version, remote_count) = extract_version(&remote_content)?;

    if local_path.exists() {
        let local_content = fs::read_to_string(local_path)?;
        match extract_version(&local_content) {
            Ok((local_version, _)) => {
                if local_version >= remote_version {
                    return Ok(UpdateStatus::AlreadyUpToDate(local_version));
                }
                let diff = compute_diff(&local_content, &remote_content);
                if let Some(parent) = local_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(local_path, &remote_content)?;
                return Ok(UpdateStatus::Updated {
                    from_version: local_version,
                    to_version: remote_version,
                    diff,
                });
            }
            Err(_) => {
                fs::remove_file(local_path)?;
            }
        }
    }

    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(local_path, &remote_content)?;
    Ok(UpdateStatus::Downloaded {
        version: remote_version,
        count: remote_count,
    })
}

// ------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_path() {
        let dir = config_dir();
        assert!(dir.to_string_lossy().ends_with(".config/wildfly-meta"));
    }

    #[test]
    fn wildfly_images_path_filename() {
        let path = wildfly_images_path();
        assert_eq!(
            path.file_name().unwrap().to_string_lossy(),
            WILDFLY_IMAGES_FILENAME
        );
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
        let tmp = std::env::temp_dir().join("wildfly-meta-test-download");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let local = tmp.join("test.toml");
        let content = "config_version = 1\nwildfly_images = []\n";
        fs::write(&local, content).unwrap();
        let version: u32 = {
            let c = fs::read_to_string(&local).unwrap();
            let config: WildFlyImagesConfig = toml::from_str(&c).unwrap();
            config.config_version
        };
        assert_eq!(version, 1);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn summary_downloaded() {
        let status = UpdateStatus::Downloaded {
            version: 5,
            count: 33,
        };
        assert_eq!(
            status.summary("WildFly images"),
            "WildFly images downloaded (33 entries, version 5)"
        );
    }

    #[test]
    fn summary_already_up_to_date() {
        let status = UpdateStatus::AlreadyUpToDate(3);
        assert_eq!(
            status.summary("Feature packs"),
            "Feature packs already up to date (version 3)"
        );
    }

    #[test]
    fn summary_updated_with_diff() {
        let status = UpdateStatus::Updated {
            from_version: 5,
            to_version: 6,
            diff: UpdateDiff {
                added: vec!["WildFly 36.0".to_string(), "WildFly 35.0.1".to_string()],
                removed: vec!["WildFly 24.0".to_string()],
            },
        };
        let summary = status.summary("WildFly images");
        assert!(summary.contains("updated from version 5 to 6"));
        assert!(summary.contains("Added: WildFly 36.0, WildFly 35.0.1"));
        assert!(summary.contains("Removed: WildFly 24.0"));
    }

    #[test]
    fn summary_updated_empty_diff() {
        let status = UpdateStatus::Updated {
            from_version: 1,
            to_version: 2,
            diff: UpdateDiff {
                added: vec![],
                removed: vec![],
            },
        };
        let summary = status.summary("WildFly images");
        assert_eq!(summary, "WildFly images updated from version 1 to 2");
    }

    #[test]
    fn update_result_summary() {
        let result = UpdateResult {
            wildfly_images: UpdateStatus::AlreadyUpToDate(5),
            feature_packs: UpdateStatus::Downloaded {
                version: 3,
                count: 7,
            },
        };
        let summary = result.summary();
        assert!(summary.contains("WildFly images already up to date (version 5)"));
        assert!(summary.contains("Feature packs downloaded (7 entries, version 3)"));
    }

    #[test]
    fn images_diff_computation() {
        let old_toml = r#"
config_version = 1

[[wildfly_images]]
major = 30
minor = 0
version = "30.0.0"
core_version = "22.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"

[[wildfly_images]]
major = 31
minor = 0
version = "31.0.0"
core_version = "23.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"
"#;

        let new_toml = r#"
config_version = 2

[[wildfly_images]]
major = 31
minor = 0
version = "31.0.0"
core_version = "23.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"

[[wildfly_images]]
major = 32
minor = 0
version = "32.0.0"
core_version = "24.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"
"#;

        let old_reg = WildFlyImageRegistry::from_toml(old_toml).unwrap();
        let new_reg = WildFlyImageRegistry::from_toml(new_toml).unwrap();
        let old_keys: BTreeSet<u16> = old_reg.keys().copied().collect();
        let new_keys: BTreeSet<u16> = new_reg.keys().copied().collect();

        let added: Vec<String> = new_keys
            .difference(&old_keys)
            .filter_map(|k| new_reg.get(*k))
            .map(|wildfly_image| wildfly_image.full_name())
            .collect();
        let removed: Vec<String> = old_keys
            .difference(&new_keys)
            .filter_map(|k| old_reg.get(*k))
            .map(|wildfly_image| wildfly_image.full_name())
            .collect();

        assert_eq!(added, vec!["WildFly 32.0"]);
        assert_eq!(removed, vec!["WildFly 30.0"]);
    }

    //noinspection DuplicatedCode
    #[test]
    fn feature_packs_diff_computation() {
        let old_toml = r#"
config_version = 1

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-feature-pack"
versions = [
  { version = "0.9.0", maven_version = "0.9.0" },
]

[[feature_packs]]
shortcut = "graphql"
name = "GraphQL"
group_id = "org.wildfly.extras.graphql"
artifact_id = "wildfly-graphql-feature-pack"
versions = [
  { version = "2.3.0", maven_version = "2.3.0.Final" },
]
"#;

        let new_toml = r#"
config_version = 2

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-feature-pack"
versions = [
  { version = "0.9.0", maven_version = "0.9.0" },
  { version = "1.0.0", maven_version = "1.0.0" },
]
"#;

        let old_reg = FeaturePackRegistry::from_toml(old_toml).unwrap();
        let new_reg = FeaturePackRegistry::from_toml(new_toml).unwrap();
        let old_keys: BTreeSet<&(String, semver::Version)> = old_reg.keys().collect();
        let new_keys: BTreeSet<&(String, semver::Version)> = new_reg.keys().collect();

        let added: Vec<String> = new_keys
            .difference(&old_keys)
            .filter_map(|(s, v)| new_reg.get(s, &v.to_string()))
            .map(|feature_pack| feature_pack.short_name())
            .collect();
        let removed: Vec<String> = old_keys
            .difference(&new_keys)
            .filter_map(|(s, v)| old_reg.get(s, &v.to_string()))
            .map(|feature_pack| feature_pack.short_name())
            .collect();

        assert_eq!(added, vec!["ai 1.0.0"]);
        assert_eq!(removed, vec!["graphql 2.3.0"]);
    }
}
