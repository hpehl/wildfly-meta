//! Galleon feature pack metadata and registry.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::registry;
use crate::update::{feature_packs_path, update_feature_packs};

const FEATURE_PACK_PORT_OFFSET_BASE: u16 = 10_000;
const FEATURE_PACK_PORT_OFFSET_STEP: u16 = 100;

/// A WildFly Galleon feature pack with Maven coordinates and version metadata.
///
/// Feature packs extend WildFly with additional capabilities (e.g. AI, GraphQL, gRPC).
/// Each feature pack has a short alias (`shortcut`) used in the version expression DSL
/// and Maven coordinates for downloading the documentation archive.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct FeaturePack {
    /// Short alias used in the DSL (e.g. `"ai"`, `"graphql"`).
    pub shortcut: String,
    /// Human-readable name (e.g. `"AI"`, `"GraphQL"`).
    pub name: String,
    /// Maven group ID (e.g. `"org.wildfly.generative-ai"`).
    pub group_id: String,
    /// Maven artifact ID (e.g. `"wildfly-ai-feature-pack"`).
    pub artifact_id: String,
    /// Zero-based index assigned to this shortcut, used for port offset computation.
    pub shortcut_index: u16,
    /// Zero-based index of this version within its shortcut group.
    pub version_index: u16,
    /// Semantic version (e.g. `0.9.0`).
    pub version: Version,
    /// Release version string, which may differ from the semantic version (e.g. `"2.7.0.Final"`).
    pub release_version: String,
}

impl FeaturePack {
    /// Returns a unique port offset for this feature pack, starting at `10_000`.
    pub fn port_offset(&self) -> u16 {
        FEATURE_PACK_PORT_OFFSET_BASE
            + (self.shortcut_index * FEATURE_PACK_PORT_OFFSET_STEP)
            + self.version_index
    }

    /// Returns a container-safe name (e.g. `"ai-0-9-0"`).
    pub fn container_name(&self) -> String {
        format!(
            "{}-{}",
            self.shortcut,
            self.version.to_string().replace('.', "-")
        )
    }

    /// Returns the Maven Central URL for the feature pack's documentation ZIP archive.
    pub fn download_url(&self) -> String {
        let group_path = self.group_id.replace('.', "/");
        format!(
            "https://repo1.maven.org/maven2/{}/{}/{}/{}-{}-doc.zip",
            group_path,
            self.artifact_id,
            self.release_version,
            self.artifact_id,
            self.release_version
        )
    }

    /// Returns a short human-readable name (e.g. `"ai 0.9.0"`).
    pub fn short_name(&self) -> String {
        format!("{} {}", self.shortcut, self.version)
    }

    /// Returns a full branded name (e.g. `"AI Feature Pack 0.9.0"`).
    pub fn full_name(&self) -> String {
        format!("{} Feature Pack {}", self.name, self.version)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct FeaturePacksConfig {
    pub config_version: u32,
    pub feature_packs: Vec<FeaturePackEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FeaturePackEntry {
    pub shortcut: String,
    pub name: String,
    pub group_id: String,
    pub artifact_id: String,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VersionEntry {
    pub version: String,
    pub release_version: String,
}

/// Registry of [`FeaturePack`] entries loaded from a TOML configuration file.
///
/// Feature packs are stored in a [`BTreeMap`] keyed by `(shortcut, version)`, so iteration
/// is alphabetical by shortcut and then by version within each shortcut group.
pub struct FeaturePackRegistry {
    feature_packs: BTreeMap<(String, Version), FeaturePack>,
}

impl FeaturePackRegistry {
    /// Loads the feature pack registry from the default configuration path
    /// (`~/.config/wildfly-meta/feature-packs.toml`).
    ///
    /// The `resolution_hint` is appended to error messages when the file is missing or
    /// unparsable, letting each consumer suggest their own recovery action
    /// (e.g. `"Run 'wado update' to fix this."`).
    pub fn load_default(resolution_hint: &str) -> Result<Self> {
        Self::load(&feature_packs_path(), resolution_hint)
    }

    /// Loads the feature pack registry, automatically downloading the configuration if it is
    /// missing or corrupt.
    ///
    /// If the configuration file does not exist, it is downloaded first. If loading fails
    /// (e.g. the file is corrupt or uses a deprecated format), the file is re-downloaded
    /// and loading is retried once.
    ///
    /// The `resolution_hint` is appended to error messages if the retry also fails.
    pub fn load_or_update(resolution_hint: &str) -> Result<Self> {
        registry::load_or_update(
            feature_packs_path(),
            resolution_hint,
            update_feature_packs,
            Self::load_default,
        )
    }

    /// Loads the feature pack registry from the given TOML file path.
    ///
    /// The `resolution_hint` is appended to error messages when the file is missing or
    /// unparsable, letting each consumer suggest their own recovery action.
    pub fn load(path: &Path, resolution_hint: &str) -> Result<Self> {
        registry::load_toml(path, resolution_hint, Self::from_toml)
    }

    /// Parses the feature pack registry from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: FeaturePacksConfig = toml::from_str(content)?;
        let mut feature_packs = BTreeMap::new();
        let mut shortcut_indices: BTreeMap<String, u16> = BTreeMap::new();
        let mut version_counts: BTreeMap<String, u16> = BTreeMap::new();

        for entry in config.feature_packs {
            let next_index = shortcut_indices.len() as u16;
            let shortcut_index = *shortcut_indices
                .entry(entry.shortcut.clone())
                .or_insert(next_index);

            for ve in entry.versions {
                let version_index = version_counts.entry(entry.shortcut.clone()).or_insert(0);
                let vi = *version_index;
                *version_index += 1;

                let version: Version = Version::parse(&ve.version)?;
                let feature_pack = FeaturePack {
                    shortcut: entry.shortcut.clone(),
                    name: entry.name.clone(),
                    group_id: entry.group_id.clone(),
                    artifact_id: entry.artifact_id.clone(),
                    shortcut_index,
                    version_index: vi,
                    version: version.clone(),
                    release_version: ve.release_version,
                };
                feature_packs.insert((entry.shortcut.clone(), version), feature_pack);
            }
        }
        Ok(Self { feature_packs })
    }

    /// Returns an iterator over `(shortcut, version)` keys in sorted order.
    pub fn keys(&self) -> impl Iterator<Item = &(String, Version)> {
        self.feature_packs.keys()
    }

    /// Returns the feature pack matching the given shortcut and version string, or `None`.
    pub fn get(&self, shortcut: &str, version: &str) -> Option<&FeaturePack> {
        let version = Version::parse(version).ok()?;
        self.feature_packs.get(&(shortcut.to_string(), version))
    }

    /// Returns the latest (last registered) version of the given shortcut, or `None`.
    pub fn latest(&self, shortcut: &str) -> Option<&FeaturePack> {
        self.feature_packs
            .iter()
            .filter(|((s, _), _)| s == shortcut)
            .map(|(_, feature_pack)| feature_pack)
            .next_back()
    }

    /// Returns the deduplicated list of known shortcut names in alphabetical order.
    pub fn known_shortcuts(&self) -> Vec<&str> {
        let mut shortcuts: Vec<&str> = self.feature_packs.keys().map(|(s, _)| s.as_str()).collect();
        shortcuts.dedup();
        shortcuts
    }

    /// Returns all known version strings for the given shortcut.
    pub fn known_versions(&self, shortcut: &str) -> Vec<String> {
        self.feature_packs
            .keys()
            .filter(|(s, _)| s == shortcut)
            .map(|(_, v)| v.to_string())
            .collect()
    }

    /// Returns all feature packs in sorted order.
    pub fn all(&self) -> Vec<&FeaturePack> {
        self.feature_packs.values().collect()
    }

    /// Returns all identifiers: bare shortcuts (e.g. `"ai"`) and versioned forms (e.g. `"ai:0.9.0"`).
    pub fn all_identifiers(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .known_shortcuts()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for (shortcut, version) in self.feature_packs.keys() {
            ids.push(format!("{shortcut}:{version}"));
        }
        ids
    }

    /// Returns the number of feature packs in the registry.
    pub fn len(&self) -> usize {
        self.feature_packs.len()
    }

    /// Returns `true` if the registry contains no feature packs.
    pub fn is_empty(&self) -> bool {
        self.feature_packs.is_empty()
    }

    /// Reads and returns the `config_version` from the given TOML file without loading the full registry.
    pub fn config_version(path: &Path) -> Result<u32> {
        registry::config_version::<FeaturePacksConfig>(path, |c| c.config_version)
    }
}

#[cfg(test)]
mod tests;
