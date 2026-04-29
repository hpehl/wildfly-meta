//! Galleon feature pack metadata and registry.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::update::feature_packs_path;

const FP_PORT_OFFSET_BASE: u16 = 10_000;

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
    /// Display version string (e.g. `"0.9.0"`).
    pub version: String,
    /// Maven version string, which may differ from the display version (e.g. `"2.7.0.Final"`).
    pub maven_version: String,
}

impl FeaturePack {
    /// Returns a unique port offset for this feature pack, starting at `10_000`.
    pub fn port_offset(&self) -> u16 {
        FP_PORT_OFFSET_BASE + (self.shortcut_index * 100) + self.version_index
    }

    /// Returns a container-safe name (e.g. `"ai-0-9-0"`).
    pub fn container_name(&self) -> String {
        format!("{}-{}", self.shortcut, self.version.replace('.', "-"))
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
    pub version: String,
    pub maven_version: String,
}

/// Registry of [`FeaturePack`] entries loaded from a TOML configuration file.
///
/// Feature packs are stored in a [`BTreeMap`] keyed by `(shortcut, version)`, so iteration
/// is alphabetical by shortcut and then by version within each shortcut group.
pub struct FeaturePackRegistry {
    packs: BTreeMap<(String, String), FeaturePack>,
}

impl FeaturePackRegistry {
    /// Loads the feature pack registry from the default configuration path (`~/.config/wildfly-meta/feature-packs.toml`).
    pub fn load_default() -> Result<Self> {
        Self::load(&feature_packs_path())
    }

    /// Loads the feature pack registry from the given TOML file path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    /// Parses the feature pack registry from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: FeaturePacksConfig = toml::from_str(content)?;
        let mut packs = BTreeMap::new();
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
            let version_index = version_counts.entry(entry.shortcut.clone()).or_insert(0);
            let vi = *version_index;
            *version_index += 1;

            let fp = FeaturePack {
                shortcut: entry.shortcut.clone(),
                name: entry.name,
                group_id: entry.group_id,
                artifact_id: entry.artifact_id,
                shortcut_index,
                version_index: vi,
                version: entry.version.clone(),
                maven_version: entry.maven_version,
            };
            packs.insert((entry.shortcut, entry.version), fp);
        }
        Ok(Self { packs })
    }

    /// Returns an iterator over `(shortcut, version)` keys in sorted order.
    pub fn keys(&self) -> impl Iterator<Item = &(String, String)> {
        self.packs.keys()
    }

    /// Returns the feature pack matching the given shortcut and version, or `None`.
    pub fn get(&self, shortcut: &str, version: &str) -> Option<&FeaturePack> {
        self.packs.get(&(shortcut.to_string(), version.to_string()))
    }

    /// Returns the latest (last registered) version of the given shortcut, or `None`.
    pub fn latest(&self, shortcut: &str) -> Option<&FeaturePack> {
        self.packs
            .iter()
            .filter(|((s, _), _)| s == shortcut)
            .map(|(_, fp)| fp)
            .next_back()
    }

    /// Returns the deduplicated list of known shortcut names in alphabetical order.
    pub fn known_shortcuts(&self) -> Vec<&str> {
        let mut shortcuts: Vec<&str> = self.packs.keys().map(|(s, _)| s.as_str()).collect();
        shortcuts.dedup();
        shortcuts
    }

    /// Returns all known version strings for the given shortcut.
    pub fn known_versions(&self, shortcut: &str) -> Vec<&str> {
        self.packs
            .keys()
            .filter(|(s, _)| s == shortcut)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// Returns all feature packs in sorted order.
    pub fn all(&self) -> Vec<&FeaturePack> {
        self.packs.values().collect()
    }

    /// Returns all identifiers: bare shortcuts (e.g. `"ai"`) and versioned forms (e.g. `"ai:0.9.0"`).
    pub fn all_identifiers(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .known_shortcuts()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for (shortcut, version) in self.packs.keys() {
            ids.push(format!("{}:{}", shortcut, version));
        }
        ids
    }

    /// Returns the number of feature packs in the registry.
    pub fn len(&self) -> usize {
        self.packs.len()
    }

    /// Returns `true` if the registry contains no feature packs.
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    /// Reads and returns the `config_version` from the given TOML file without loading the full registry.
    pub fn config_version(path: &Path) -> Result<u32> {
        let content = fs::read_to_string(path)?;
        let config: FeaturePacksConfig = toml::from_str(&content)?;
        Ok(config.config_version)
    }
}

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
        assert_eq!(reg.len(), 5);
    }

    #[test]
    fn load_from_path() {
        let tmp = std::env::temp_dir().join("wildfly-meta-test-fp-load");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("feature-packs.toml");
        let content = include_str!("../feature-packs.toml");
        fs::write(&path, content).unwrap();

        let reg = FeaturePackRegistry::load(&path).unwrap();
        assert_eq!(reg.len(), 5);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_from_missing_path() {
        let path = Path::new("/nonexistent/feature-packs.toml");
        assert!(FeaturePackRegistry::load(path).is_err());
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
        let fp = reg.get("ai", "0.9.0").unwrap();
        assert_eq!(fp.name, "AI");
        assert_eq!(fp.group_id, "org.wildfly.generative-ai");
    }

    #[test]
    fn get_unknown() {
        let reg = test_registry();
        assert!(reg.get("unknown", "1.0.0").is_none());
    }

    #[test]
    fn latest_version() {
        let reg = test_registry();
        let fp = reg.latest("ai").unwrap();
        assert_eq!(fp.version, "0.9.0");
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
        assert_eq!(
            shortcuts,
            vec!["ai", "graphql", "grpc", "keycloak", "myfaces"]
        );
    }

    #[test]
    fn known_versions_for_shortcut() {
        let reg = test_registry();
        let versions = reg.known_versions("ai");
        assert_eq!(versions, vec!["0.9.0"]);
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
feature_packs = []
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
        assert_eq!(reg.get("ai", "0.9.0").unwrap().version_index, 0);
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
version = "0.8.0"
maven_version = "0.8.0"

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-fp"
version = "0.9.0"
maven_version = "0.9.0"
"#;
        let reg = FeaturePackRegistry::from_toml(toml).unwrap();
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.get("ai", "0.8.0").unwrap().version_index, 0);
        assert_eq!(reg.get("ai", "0.9.0").unwrap().version_index, 1);
        assert_eq!(reg.get("ai", "0.8.0").unwrap().shortcut_index, 0);
        assert_eq!(reg.get("ai", "0.9.0").unwrap().shortcut_index, 0);
        let latest = reg.latest("ai").unwrap();
        assert_eq!(latest.version, "0.9.0");
        assert_eq!(reg.known_versions("ai"), vec!["0.8.0", "0.9.0"]);
    }

    // ------------------------------------------------------ feature pack methods

    #[test]
    fn port_offset() {
        let reg = test_registry();
        assert_eq!(reg.get("ai", "0.9.0").unwrap().port_offset(), 10_000);
        assert_eq!(reg.get("graphql", "2.7.0").unwrap().port_offset(), 10_100);
        assert_eq!(reg.get("grpc", "0.1.16").unwrap().port_offset(), 10_200);
        assert_eq!(reg.get("keycloak", "26.6.1").unwrap().port_offset(), 10_300);
        assert_eq!(reg.get("myfaces", "2.0.3").unwrap().port_offset(), 10_400);
    }

    #[test]
    fn unique_port_offsets() {
        let reg = test_registry();
        let mut offsets: Vec<u16> = reg.all().iter().map(|fp| fp.port_offset()).collect();
        let len = offsets.len();
        offsets.sort();
        offsets.dedup();
        assert_eq!(len, offsets.len());
    }

    #[test]
    fn port_offsets_start_at_10000() {
        let reg = test_registry();
        for fp in reg.all() {
            assert!(fp.port_offset() >= 10_000);
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
        let fp = reg.get("ai", "0.9.0").unwrap();
        assert_eq!(
            fp.download_url(),
            "https://repo1.maven.org/maven2/org/wildfly/generative-ai/wildfly-ai-feature-pack/0.9.0/wildfly-ai-feature-pack-0.9.0-doc.zip"
        );
    }

    #[test]
    fn download_url_with_final() {
        let reg = test_registry();
        let fp = reg.get("graphql", "2.7.0").unwrap();
        assert_eq!(
            fp.download_url(),
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
