use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::feature_pack::{FeaturePackRegistry, FeaturePacksConfig};
use crate::image::{ImageRegistry, WildFlyImagesConfig};

const DEFAULT_BASE_URL: &str = "https://raw.githubusercontent.com/hpehl/wildfly-meta/main";

pub const IMAGES_FILENAME: &str = "wildfly-images.toml";
pub const FEATURE_PACKS_FILENAME: &str = "feature-packs.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Downloaded {
        version: u32,
        count: usize,
    },
    Updated {
        from_version: u32,
        to_version: u32,
        diff: UpdateDiff,
    },
    AlreadyUpToDate(u32),
}

impl UpdateStatus {
    fn summary(&self, label: &str) -> String {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateResult {
    pub images: UpdateStatus,
    pub feature_packs: UpdateStatus,
}

impl UpdateResult {
    pub fn summary(&self) -> String {
        let images = self.images.summary("WildFly images");
        let packs = self.feature_packs.summary("Feature packs");
        format!("{images}\n{packs}")
    }
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

pub fn update_all() -> Result<UpdateResult> {
    update_all_with_base_url(DEFAULT_BASE_URL)
}

pub fn update_all_with_base_url(base_url: &str) -> Result<UpdateResult> {
    let images = update_images_with_base_url(base_url)?;
    let packs = update_feature_packs_with_base_url(base_url)?;
    Ok(UpdateResult {
        images,
        feature_packs: packs,
    })
}

pub fn update_images() -> Result<UpdateStatus> {
    update_images_with_base_url(DEFAULT_BASE_URL)
}

pub fn update_images_with_base_url(base_url: &str) -> Result<UpdateStatus> {
    let url = format!("{}/{}", base_url, IMAGES_FILENAME);
    let local_path = images_path();
    update_file(
        &url,
        &local_path,
        |content| {
            let config: WildFlyImagesConfig = toml::from_str(content)?;
            Ok((config.config_version, config.images.len()))
        },
        |old_content, new_content| {
            let old_registry = ImageRegistry::from_toml(old_content);
            let new_registry = ImageRegistry::from_toml(new_content);
            match (old_registry, new_registry) {
                (Ok(old_reg), Ok(new_reg)) => {
                    let old_keys: BTreeSet<u16> = old_reg.keys().copied().collect();
                    let new_keys: BTreeSet<u16> = new_reg.keys().copied().collect();
                    let added = new_keys
                        .difference(&old_keys)
                        .filter_map(|k| new_reg.get(*k))
                        .map(|img| format!("WildFly {}", img.display_version()))
                        .collect();
                    let removed = old_keys
                        .difference(&new_keys)
                        .filter_map(|k| old_reg.get(*k))
                        .map(|img| format!("WildFly {}", img.display_version()))
                        .collect();
                    UpdateDiff { added, removed }
                }
                _ => UpdateDiff {
                    added: vec![],
                    removed: vec![],
                },
            }
        },
    )
}

pub fn update_feature_packs() -> Result<UpdateStatus> {
    update_feature_packs_with_base_url(DEFAULT_BASE_URL)
}

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
            let old_registry = FeaturePackRegistry::from_toml(old_content);
            let new_registry = FeaturePackRegistry::from_toml(new_content);
            match (old_registry, new_registry) {
                (Ok(old_reg), Ok(new_reg)) => {
                    let old_keys: BTreeSet<&(String, String)> = old_reg.keys().collect();
                    let new_keys: BTreeSet<&(String, String)> = new_reg.keys().collect();
                    let added = new_keys
                        .difference(&old_keys)
                        .filter_map(|(s, v)| new_reg.get(s, v))
                        .map(|fp| fp.display_name())
                        .collect();
                    let removed = old_keys
                        .difference(&new_keys)
                        .filter_map(|(s, v)| old_reg.get(s, v))
                        .map(|fp| fp.display_name())
                        .collect();
                    UpdateDiff { added, removed }
                }
                _ => UpdateDiff {
                    added: vec![],
                    removed: vec![],
                },
            }
        },
    )
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
        let (local_version, _) = extract_version(&local_content)?;
        if local_version >= remote_version {
            return Ok(UpdateStatus::AlreadyUpToDate(local_version));
        }
        let diff = compute_diff(&local_content, &remote_content);
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(local_path, &remote_content)?;
        Ok(UpdateStatus::Updated {
            from_version: local_version,
            to_version: remote_version,
            diff,
        })
    } else {
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(local_path, &remote_content)?;
        Ok(UpdateStatus::Downloaded {
            version: remote_version,
            count: remote_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let tmp = std::env::temp_dir().join("wildfly-meta-test-download");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let local = tmp.join("test.toml");
        let content = "config_version = 1\nimages = []\n";
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
            images: UpdateStatus::AlreadyUpToDate(5),
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

[[images]]
major = 30
minor = 0
version = "30.0.0"
core_version = "22.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"

[[images]]
major = 31
minor = 0
version = "31.0.0"
core_version = "23.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"
"#;

        let new_toml = r#"
config_version = 2

[[images]]
major = 31
minor = 0
version = "31.0.0"
core_version = "23.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"

[[images]]
major = 32
minor = 0
version = "32.0.0"
core_version = "24.0.0"
suffix = "Final"
repository = "quay.io/wildfly/wildfly"
"#;

        let old_reg = ImageRegistry::from_toml(old_toml).unwrap();
        let new_reg = ImageRegistry::from_toml(new_toml).unwrap();
        let old_keys: BTreeSet<u16> = old_reg.keys().copied().collect();
        let new_keys: BTreeSet<u16> = new_reg.keys().copied().collect();

        let added: Vec<String> = new_keys
            .difference(&old_keys)
            .filter_map(|k| new_reg.get(*k))
            .map(|img| format!("WildFly {}", img.display_version()))
            .collect();
        let removed: Vec<String> = old_keys
            .difference(&new_keys)
            .filter_map(|k| old_reg.get(*k))
            .map(|img| format!("WildFly {}", img.display_version()))
            .collect();

        assert_eq!(added, vec!["WildFly 32.0"]);
        assert_eq!(removed, vec!["WildFly 30.0"]);
    }

    #[test]
    fn feature_packs_diff_computation() {
        let old_toml = r#"
config_version = 1

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-feature-pack"
version = "0.9.0"
maven_version = "0.9.0"

[[feature_packs]]
shortcut = "graphql"
name = "GraphQL"
group_id = "org.wildfly.extras.graphql"
artifact_id = "wildfly-graphql-feature-pack"
version = "2.3.0"
maven_version = "2.3.0.Final"
"#;

        let new_toml = r#"
config_version = 2

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-feature-pack"
version = "0.9.0"
maven_version = "0.9.0"

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-feature-pack"
version = "1.0.0"
maven_version = "1.0.0"
"#;

        let old_reg = FeaturePackRegistry::from_toml(old_toml).unwrap();
        let new_reg = FeaturePackRegistry::from_toml(new_toml).unwrap();
        let old_keys: BTreeSet<&(String, String)> = old_reg.keys().collect();
        let new_keys: BTreeSet<&(String, String)> = new_reg.keys().collect();

        let added: Vec<String> = new_keys
            .difference(&old_keys)
            .filter_map(|(s, v)| new_reg.get(s, v))
            .map(|fp| fp.display_name())
            .collect();
        let removed: Vec<String> = old_keys
            .difference(&new_keys)
            .filter_map(|(s, v)| old_reg.get(s, v))
            .map(|fp| fp.display_name())
            .collect();

        assert_eq!(added, vec!["ai 1.0.0"]);
        assert_eq!(removed, vec!["graphql 2.3.0"]);
    }
}
