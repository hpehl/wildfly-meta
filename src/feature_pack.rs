//! Galleon feature pack metadata and registry.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use semver::Version;
use serde::Deserialize;

use crate::update::feature_packs_path;

const FEATURE_PACK_PORT_OFFSET_BASE: u16 = 10_000;
// Max versions per shortcut before colliding with the next shortcut's port range
const FEATURE_PACK_PORT_OFFSET_STEP: u16 = 100;

/// A WildFly Galleon feature pack with Maven coordinates and version metadata.
///
/// Feature packs extend WildFly with additional capabilities (e.g. AI, GraphQL, gRPC).
/// Each feature pack has a short alias (`shortcut`) used in the version expression DSL
/// and Maven coordinates for downloading the documentation archive.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// Maven version string, which may differ from the semantic version (e.g. `"2.7.0.Final"`).
    pub maven_version: String,
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
            group_path, self.artifact_id, self.maven_version, self.artifact_id, self.maven_version
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
    pub maven_version: String,
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

    /// Loads the feature pack registry from the given TOML file path.
    ///
    /// The `resolution_hint` is appended to error messages when the file is missing or
    /// unparsable, letting each consumer suggest their own recovery action.
    pub fn load(path: &Path, resolution_hint: &str) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            if resolution_hint.is_empty() {
                anyhow::anyhow!("{e}")
            } else {
                anyhow::anyhow!("{e}. {resolution_hint}")
            }
        })?;
        Self::from_toml(&content).map_err(|e| {
            if resolution_hint.is_empty() {
                anyhow::anyhow!("Failed to parse {}: {e}", path.display())
            } else {
                anyhow::anyhow!("Failed to parse {}: {e}. {resolution_hint}", path.display())
            }
        })
    }

    /// Parses the feature pack registry from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: FeaturePacksConfig = toml::from_str(content)?;
        let mut feature_packs = BTreeMap::new();
        let mut shortcut_indices: BTreeMap<String, u16> = BTreeMap::new();
        let mut version_counts: BTreeMap<String, u16> = BTreeMap::new();
        let mut next_shortcut_index: u16 = 0;

        for entry in config.feature_packs {
            let shortcut_index = match shortcut_indices.get(&entry.shortcut) {
                Some(&idx) => idx,
                None => {
                    let idx = next_shortcut_index;
                    shortcut_indices.insert(entry.shortcut.clone(), idx);
                    next_shortcut_index += 1;
                    idx
                }
            };

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
                    maven_version: ve.maven_version,
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
        let content = fs::read_to_string(path)?;
        let config: FeaturePacksConfig = toml::from_str(&content)?;
        Ok(config.config_version)
    }
}

// ------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> FeaturePackRegistry {
        let toml = include_str!("../feature-packs.toml");
        FeaturePackRegistry::from_toml(toml).expect("failed to parse feature-packs.toml")
    }

    // ------------------------------------------------------ loading & config_version

    #[test]
    fn load_all_packs() {
        let reg = test_registry();
        assert!(!reg.is_empty());
    }

    #[test]
    fn load_from_path() {
        let tmp = std::env::temp_dir().join("wildfly-meta-test-fp-load");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("feature-packs.toml");
        let content = include_str!("../feature-packs.toml");
        fs::write(&path, content).unwrap();

        let reg = FeaturePackRegistry::load(&path, "").unwrap();
        assert!(!reg.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_from_missing_path() {
        let path = Path::new("/nonexistent/feature-packs.toml");
        assert!(FeaturePackRegistry::load(path, "").is_err());
    }

    #[test]
    fn load_missing_file_includes_resolution_hint() {
        let path = Path::new("/nonexistent/feature-packs.toml");
        let hint = "Run 'mytool update' to fix this.";
        let result = FeaturePackRegistry::load(path, hint);
        assert!(result.is_err());
        let err = format!("{}", result.err().unwrap());
        assert!(err.contains(hint));
    }

    #[test]
    fn load_corrupt_file_includes_resolution_hint() {
        let tmp = std::env::temp_dir().join("wildfly-meta-test-fp-corrupt");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("feature-packs.toml");
        fs::write(&path, "this is not valid toml {{{}}}").unwrap();

        let hint = "Run 'mytool update' to fix this.";
        let result = FeaturePackRegistry::load(&path, hint);
        assert!(result.is_err());
        let err = format!("{}", result.err().unwrap());
        assert!(err.contains(hint));
        assert!(err.contains("Failed to parse"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_empty_resolution_hint_no_trailing_dot() {
        let path = Path::new("/nonexistent/feature-packs.toml");
        let result = FeaturePackRegistry::load(path, "");
        assert!(result.is_err());
        let err = format!("{}", result.err().unwrap());
        assert!(!err.ends_with(". "));
    }

    #[test]
    fn config_version_from_file() {
        let tmp = std::env::temp_dir().join("wildfly-meta-test-fp-cv");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("feature-packs.toml");
        let content = include_str!("../feature-packs.toml");
        fs::write(&path, content).unwrap();

        let version = FeaturePackRegistry::config_version(&path).unwrap();
        assert!(version >= 1);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn config_version_missing_file() {
        let path = Path::new("/nonexistent/feature-packs.toml");
        assert!(FeaturePackRegistry::config_version(path).is_err());
    }

    // ------------------------------------------------------ registry queries

    #[test]
    fn get_by_shortcut_version() {
        let reg = test_registry();
        let latest = reg.latest("ai").unwrap();
        let feature_pack = reg.get("ai", &latest.version.to_string()).unwrap();
        assert_eq!(feature_pack.shortcut, "ai");
        assert_eq!(feature_pack.version, latest.version);
    }

    #[test]
    fn get_unknown() {
        let reg = test_registry();
        assert!(reg.get("unknown", "1.0.0").is_none());
    }

    #[test]
    fn latest_version() {
        let reg = test_registry();
        let shortcuts = reg.known_shortcuts();
        for shortcut in &shortcuts {
            let feature_pack = reg.latest(shortcut).unwrap();
            assert!(!feature_pack.version.to_string().is_empty());
            assert_eq!(feature_pack.shortcut, *shortcut);
        }
    }

    #[test]
    fn latest_unknown() {
        let reg = test_registry();
        assert!(reg.latest("unknown").is_none());
    }

    #[test]
    fn known_shortcuts() {
        let reg = test_registry();
        let shortcuts = reg.known_shortcuts();
        assert!(!shortcuts.is_empty());
        let mut sorted = shortcuts.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(shortcuts, sorted, "shortcuts should be sorted and unique");
    }

    #[test]
    fn known_versions_for_shortcut() {
        let reg = test_registry();
        let versions = reg.known_versions("ai");
        assert_eq!(versions, vec!["0.8.1", "0.9.0", "0.9.1"]);
    }

    #[test]
    fn known_versions_unknown() {
        let reg = test_registry();
        assert!(reg.known_versions("unknown").is_empty());
    }

    #[test]
    fn all_identifiers() {
        let reg = test_registry();
        let ids = reg.all_identifiers();
        assert!(ids.contains(&"ai".to_string()));
        assert!(ids.contains(&"ai:0.9.0".to_string()));
        assert!(ids.contains(&"grpc".to_string()));
        assert!(ids.contains(&"grpc:0.1.16".to_string()));
    }

    #[test]
    fn keys_returns_sorted() {
        let reg = test_registry();
        let keys: Vec<_> = reg.keys().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn all_returns_all_packs() {
        let reg = test_registry();
        assert_eq!(reg.all().len(), reg.len());
    }

    #[test]
    fn is_empty_false() {
        let reg = test_registry();
        assert!(!reg.is_empty());
    }

    #[test]
    fn is_empty_true() {
        let toml = r#"
config_version = 1

[[feature_packs]]
shortcut = "empty"
name = "Empty"
group_id = "org.example"
artifact_id = "empty-fp"
versions = []
"#;
        let reg = FeaturePackRegistry::from_toml(toml).unwrap();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    // ------------------------------------------------------ index computation

    #[test]
    fn shortcut_index_computed() {
        let reg = test_registry();
        assert_eq!(reg.get("ai", "0.9.0").unwrap().shortcut_index, 0);
        assert_eq!(reg.get("graphql", "2.7.0").unwrap().shortcut_index, 1);
        assert_eq!(reg.get("grpc", "0.1.16").unwrap().shortcut_index, 2);
        assert_eq!(reg.get("keycloak", "26.6.1").unwrap().shortcut_index, 3);
        assert_eq!(reg.get("myfaces", "2.0.3").unwrap().shortcut_index, 4);
    }

    #[test]
    fn version_index_computed() {
        let reg = test_registry();
        assert_eq!(reg.get("ai", "0.9.1").unwrap().version_index, 0);
        assert_eq!(reg.get("ai", "0.9.0").unwrap().version_index, 1);
        assert_eq!(reg.get("ai", "0.8.1").unwrap().version_index, 2);
    }

    #[test]
    fn multiple_versions_per_shortcut() {
        let toml = r#"
config_version = 1

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-fp"
versions = [
  { version = "0.8.0", maven_version = "0.8.0" },
  { version = "0.9.0", maven_version = "0.9.0" },
]
"#;
        let reg = FeaturePackRegistry::from_toml(toml).unwrap();
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.get("ai", "0.8.0").unwrap().version_index, 0);
        assert_eq!(reg.get("ai", "0.9.0").unwrap().version_index, 1);
        assert_eq!(reg.get("ai", "0.8.0").unwrap().shortcut_index, 0);
        assert_eq!(reg.get("ai", "0.9.0").unwrap().shortcut_index, 0);
        let latest = reg.latest("ai").unwrap();
        assert_eq!(latest.version.to_string(), "0.9.0");
        assert_eq!(reg.known_versions("ai"), vec!["0.8.0", "0.9.0"]);
    }

    // ------------------------------------------------------ feature pack methods

    #[test]
    fn port_offset() {
        let reg = test_registry();
        assert_eq!(reg.get("ai", "0.9.1").unwrap().port_offset(), 10_000);
        assert_eq!(reg.get("graphql", "2.7.0").unwrap().port_offset(), 10_100);
        assert_eq!(reg.get("grpc", "0.1.16").unwrap().port_offset(), 10_200);
        assert_eq!(reg.get("keycloak", "26.6.1").unwrap().port_offset(), 10_300);
        assert_eq!(reg.get("myfaces", "2.0.3").unwrap().port_offset(), 10_400);
    }

    #[test]
    fn unique_port_offsets() {
        let reg = test_registry();
        let mut offsets: Vec<u16> = reg
            .all()
            .iter()
            .map(|feature_pack| feature_pack.port_offset())
            .collect();
        let len = offsets.len();
        offsets.sort();
        offsets.dedup();
        assert_eq!(len, offsets.len());
    }

    #[test]
    fn port_offsets_start_at_10000() {
        let reg = test_registry();
        for feature_pack in reg.all() {
            assert!(feature_pack.port_offset() >= 10_000);
        }
    }

    #[test]
    fn container_name() {
        let reg = test_registry();
        assert_eq!(reg.get("ai", "0.9.0").unwrap().container_name(), "ai-0-9-0");
        assert_eq!(
            reg.get("graphql", "2.7.0").unwrap().container_name(),
            "graphql-2-7-0"
        );
    }

    #[test]
    fn download_url_without_final() {
        let reg = test_registry();
        let feature_pack = reg.get("ai", "0.9.0").unwrap();
        assert_eq!(
            feature_pack.download_url(),
            "https://repo1.maven.org/maven2/org/wildfly/generative-ai/wildfly-ai-feature-pack/0.9.0/wildfly-ai-feature-pack-0.9.0-doc.zip"
        );
    }

    #[test]
    fn download_url_with_final() {
        let reg = test_registry();
        let feature_pack = reg.get("graphql", "2.7.0").unwrap();
        assert_eq!(
            feature_pack.download_url(),
            "https://repo1.maven.org/maven2/org/wildfly/extras/graphql/wildfly-microprofile-graphql-feature-pack/2.7.0.Final/wildfly-microprofile-graphql-feature-pack-2.7.0.Final-doc.zip"
        );
    }

    #[test]
    fn short_name() {
        let reg = test_registry();
        assert_eq!(reg.get("ai", "0.9.0").unwrap().short_name(), "ai 0.9.0");
        assert_eq!(
            reg.get("grpc", "0.1.16").unwrap().short_name(),
            "grpc 0.1.16"
        );
    }
}
