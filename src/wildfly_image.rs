//! WildFly container image metadata and registry.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use semver::Version;
use serde::Deserialize;

use crate::update::wildfly_images_path;

/// The version string used to refer to the WildFly development build (e.g. `"dev"`).
pub static DEVELOPMENT_VERSION: &str = "dev";

/// The Docker tag used for the WildFly development image.
pub static DEVELOPMENT_TAG: &str = "development";

const HTTP_PORT_BASE: u16 = 8000;
const MANAGEMENT_PORT_BASE: u16 = 9000;

/// Returns a [`WildFlyImage`] representing the WildFly development build.
///
/// The development image has an identifier of `0` and builds from the WildFly Git repository
/// instead of pulling a container image.
pub fn wildfly_dev() -> WildFlyImage {
    WildFlyImage {
        identifier: 0,
        version: Version::new(0, 0, 0),
        short_version: String::new(),
        core_version: Version::new(0, 0, 0),
        suffix: String::new(),
        repository: String::new(),
        platforms: vec![],
    }
}

/// A WildFly container image with version and registry metadata.
///
/// Each image corresponds to a specific WildFly release and contains the information needed to
/// pull the container image, compute unique port offsets, and identify the release.
///
/// Images are ordered by their [`identifier`](WildFlyImage::identifier), which encodes
/// `major * 10 + minor` (e.g. `261` for WildFly 26.1).
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct WildFlyImage {
    /// Numeric identifier encoding `major * 10 + minor` (e.g. `340` for WildFly 34.0).
    pub identifier: u16,
    /// Full semantic version of the WildFly release (e.g. `34.0.1`).
    pub version: Version,
    /// Short version string for display (e.g. `"34.0"` or `"26.1"`).
    pub short_version: String,
    /// WildFly Core version bundled with this release.
    pub core_version: Version,
    /// Container image tag suffix (e.g. `"Final"` or `"Final-jdk17"`).
    pub suffix: String,
    /// Container registry and repository (e.g. `"quay.io/wildfly/wildfly"`).
    pub repository: String,
    /// Supported platform architectures (e.g. `["linux/amd64", "linux/arm64"]`).
    pub platforms: Vec<String>,
}

impl WildFlyImage {
    /// Returns the full container image reference (e.g. `"quay.io/wildfly/wildfly:34.0.1.Final"`),
    /// or the WildFly Git repository URL for the development build.
    pub fn image_ref(&self) -> String {
        if self.is_dev() {
            "https://github.com/wildfly/wildfly.git".to_string()
        } else {
            format!("{}:{}.{}", self.repository, self.version, self.suffix)
        }
    }

    /// Returns `true` if this is the development build (identifier `0`).
    pub fn is_dev(&self) -> bool {
        self.identifier == 0
    }

    /// Returns a short human-readable name: `"dev"` for development, or `"34.0"` / `"26.1"` for releases.
    pub fn short_name(&self) -> String {
        if self.is_dev() {
            DEVELOPMENT_VERSION.to_string()
        } else {
            self.short_version.clone()
        }
    }

    /// Returns a full branded name: `"WildFly dev"` for development, or `"WildFly 34.0"` / `"WildFly 26.1"` for releases.
    pub fn full_name(&self) -> String {
        format!("WildFly {}", self.short_name())
    }

    /// Returns the HTTP port for this image (base `8000` + identifier as port offset).
    pub fn http_port(&self) -> u16 {
        HTTP_PORT_BASE
            .checked_add(self.identifier)
            .expect("HTTP port overflow")
    }

    /// Returns the management port for this image (base `9000` + identifier as port offset).
    pub fn management_port(&self) -> u16 {
        MANAGEMENT_PORT_BASE
            .checked_add(self.identifier)
            .expect("management port overflow")
    }
}

impl Ord for WildFlyImage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.identifier.cmp(&other.identifier)
    }
}

impl PartialOrd for WildFlyImage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct WildFlyImagesConfig {
    pub config_version: u32,
    pub wildfly_images: Vec<WildFlyImageEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WildFlyImageEntry {
    pub major: u16,
    pub minor: u16,
    pub version: Version,
    pub core_version: Version,
    pub suffix: String,
    pub repository: String,
    #[serde(default)]
    pub platforms: Vec<String>,
}

/// Registry of [`WildFlyImage`] entries loaded from a TOML configuration file.
///
/// Images are stored in a [`BTreeMap`] keyed by their numeric identifier, so iteration
/// is always in version order (oldest to newest).
pub struct WildFlyImageRegistry {
    wildfly_images: BTreeMap<u16, WildFlyImage>,
}

impl WildFlyImageRegistry {
    /// Loads the image registry from the default configuration path (`~/.config/wildfly-meta/wildfly-images.toml`).
    pub fn load_default() -> Result<Self> {
        Self::load(&wildfly_images_path())
    }

    /// Loads the image registry from the given TOML file path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    /// Parses the image registry from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: WildFlyImagesConfig = toml::from_str(content)?;
        let mut wildfly_images = BTreeMap::new();
        for entry in config.wildfly_images {
            let id = identifier(entry.major, entry.minor);
            let wildfly_image = WildFlyImage {
                identifier: id,
                short_version: format!("{}.{}", entry.major, entry.minor),
                version: entry.version,
                core_version: entry.core_version,
                suffix: entry.suffix,
                repository: entry.repository,
                platforms: entry.platforms,
            };
            wildfly_images.insert(id, wildfly_image);
        }
        Ok(Self { wildfly_images })
    }

    /// Returns the image with the given identifier, or `None` if not found.
    pub fn get(&self, id: u16) -> Option<&WildFlyImage> {
        self.wildfly_images.get(&id)
    }

    /// Returns the oldest (lowest identifier) image in the registry.
    pub fn first(&self) -> Option<&WildFlyImage> {
        self.wildfly_images.first_key_value().map(|(_, v)| v)
    }

    /// Returns the newest (highest identifier) image in the registry.
    pub fn last(&self) -> Option<&WildFlyImage> {
        self.wildfly_images.last_key_value().map(|(_, v)| v)
    }

    /// Returns all images with identifiers in the inclusive range `[from, to]`.
    pub fn range(&self, from: u16, to: u16) -> Vec<&WildFlyImage> {
        self.wildfly_images
            .range(from..=to)
            .map(|(_, v)| v)
            .collect()
    }

    /// Returns all images in version order.
    pub fn all(&self) -> Vec<&WildFlyImage> {
        self.wildfly_images.values().collect()
    }

    /// Returns the number of images in the registry.
    pub fn len(&self) -> usize {
        self.wildfly_images.len()
    }

    /// Returns `true` if the registry contains no images.
    pub fn is_empty(&self) -> bool {
        self.wildfly_images.is_empty()
    }

    /// Returns an iterator over the image identifiers in ascending order.
    pub fn keys(&self) -> impl Iterator<Item = &u16> {
        self.wildfly_images.keys()
    }

    /// Reads and returns the `config_version` from the given TOML file without loading the full registry.
    pub fn config_version(path: &Path) -> Result<u32> {
        let content = fs::read_to_string(path)?;
        let config: WildFlyImagesConfig = toml::from_str(&content)?;
        Ok(config.config_version)
    }
}

/// Computes the numeric identifier for a WildFly version: `major * 10 + minor`.
///
/// For example, WildFly 26.1 has identifier `261`, and WildFly 34.0 has identifier `340`.
pub fn identifier(major: u16, minor: u16) -> u16 {
    major * 10 + minor
}

/// Extracts the major version from an identifier (e.g. `340` → `34`).
pub fn identifier_major(id: u16) -> u16 {
    id / 10
}

/// Extracts the minor version from an identifier (e.g. `261` → `1`).
pub fn identifier_minor(id: u16) -> u16 {
    id % 10
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> WildFlyImageRegistry {
        let toml = include_str!("../wildfly-images.toml");
        WildFlyImageRegistry::from_toml(toml).expect("failed to parse wildfly-images.toml")
    }

    // ------------------------------------------------------ loading & config_version

    #[test]
    fn load_all_images() {
        let reg = test_registry();
        assert_eq!(reg.len(), 33);
    }

    #[test]
    fn load_from_path() {
        let tmp = std::env::temp_dir().join("wildfly-meta-test-wildfly-image-load");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("wildfly-images.toml");
        let content = include_str!("../wildfly-images.toml");
        fs::write(&path, content).unwrap();

        let reg = WildFlyImageRegistry::load(&path).unwrap();
        assert_eq!(reg.len(), 33);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_from_missing_path() {
        let path = Path::new("/nonexistent/wildfly-images.toml");
        assert!(WildFlyImageRegistry::load(path).is_err());
    }

    #[test]
    fn config_version_from_file() {
        let tmp = std::env::temp_dir().join("wildfly-meta-test-wildfly-image-cv");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("wildfly-images.toml");
        let content = include_str!("../wildfly-images.toml");
        fs::write(&path, content).unwrap();

        let version = WildFlyImageRegistry::config_version(&path).unwrap();
        assert!(version >= 1);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn config_version_missing_file() {
        let path = Path::new("/nonexistent/wildfly-images.toml");
        assert!(WildFlyImageRegistry::config_version(path).is_err());
    }

    // ------------------------------------------------------ registry queries

    #[test]
    fn get_by_identifier() {
        let reg = test_registry();
        let wildfly_image = reg.get(261).unwrap();
        assert_eq!(wildfly_image.short_version, "26.1");
        assert_eq!(wildfly_image.suffix, "Final-jdk17");
    }

    #[test]
    fn get_unknown() {
        let reg = test_registry();
        assert!(reg.get(999).is_none());
    }

    #[test]
    fn first_image() {
        let reg = test_registry();
        let first = reg.first().unwrap();
        assert_eq!(first.identifier, 100);
        assert_eq!(first.short_version, "10.0");
    }

    #[test]
    fn last_image() {
        let reg = test_registry();
        let last = reg.last().unwrap();
        assert_eq!(last.identifier, 390);
        assert_eq!(last.short_version, "39.0");
    }

    #[test]
    fn first_and_last_on_empty() {
        let toml = r#"
config_version = 1
wildfly_images = []
"#;
        let reg = WildFlyImageRegistry::from_toml(toml).unwrap();
        assert!(reg.first().is_none());
        assert!(reg.last().is_none());
    }

    #[test]
    fn range_query() {
        let reg = test_registry();
        let wildfly_images = reg.range(200, 220);
        assert_eq!(wildfly_images.len(), 3); // 20.0, 21.0, 22.0
    }

    #[test]
    fn range_empty_result() {
        let reg = test_registry();
        let wildfly_images = reg.range(500, 600);
        assert!(wildfly_images.is_empty());
    }

    #[test]
    fn keys_returns_sorted() {
        let reg = test_registry();
        let keys: Vec<_> = reg.keys().copied().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn all_returns_all_images() {
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
wildfly_images = []
"#;
        let reg = WildFlyImageRegistry::from_toml(toml).unwrap();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    // ------------------------------------------------------ identifier helpers

    #[test]
    fn identifier_roundtrip() {
        assert_eq!(identifier(34, 0), 340);
        assert_eq!(identifier_major(340), 34);
        assert_eq!(identifier_minor(340), 0);
        assert_eq!(identifier(26, 1), 261);
        assert_eq!(identifier_major(261), 26);
        assert_eq!(identifier_minor(261), 1);
    }

    // ------------------------------------------------------ wildfly_dev

    #[test]
    fn wildfly_dev_fields() {
        let dev = wildfly_dev();
        assert_eq!(dev.identifier, 0);
        assert!(dev.is_dev());
        assert_eq!(dev.http_port(), 8000);
        assert_eq!(dev.management_port(), 9000);
        assert!(dev.short_version.is_empty());
        assert!(dev.platforms.is_empty());
    }

    #[test]
    fn is_dev() {
        let dev = wildfly_dev();
        assert!(dev.is_dev());
        let reg = test_registry();
        assert!(!reg.get(100).unwrap().is_dev());
    }

    // ------------------------------------------------------ wildfly image methods

    #[test]
    fn short_name_regular() {
        let reg = test_registry();
        let wildfly_image = reg.get(250).unwrap();
        assert_eq!(wildfly_image.short_name(), "25.0");
        let wildfly_image = reg.get(261).unwrap();
        assert_eq!(wildfly_image.short_name(), "26.1");
    }

    #[test]
    fn short_name_dev() {
        let dev = wildfly_dev();
        assert_eq!(dev.short_name(), "dev");
    }

    #[test]
    fn full_name_regular() {
        let reg = test_registry();
        let wildfly_image = reg.get(250).unwrap();
        assert_eq!(wildfly_image.full_name(), "WildFly 25.0");
        let wildfly_image = reg.get(261).unwrap();
        assert_eq!(wildfly_image.full_name(), "WildFly 26.1");
    }

    #[test]
    fn full_name_dev() {
        let dev = wildfly_dev();
        assert_eq!(dev.full_name(), "WildFly dev");
    }

    #[test]
    fn image_ref_regular() {
        let reg = test_registry();
        let wildfly_image = reg.get(390).unwrap();
        assert!(wildfly_image
            .image_ref()
            .starts_with("quay.io/wildfly/wildfly:"));
    }

    #[test]
    fn image_ref_dev() {
        let dev = wildfly_dev();
        assert_eq!(dev.image_ref(), "https://github.com/wildfly/wildfly.git");
    }

    #[test]
    fn image_ref_includes_suffix() {
        let reg = test_registry();
        let wildfly_image = reg.get(261).unwrap();
        let name = wildfly_image.image_ref();
        assert!(name.contains("Final-jdk17"));
    }

    #[test]
    fn http_port() {
        let reg = test_registry();
        let wildfly_image = reg.get(340).unwrap();
        assert_eq!(wildfly_image.http_port(), 8340);
    }

    #[test]
    fn management_port() {
        let reg = test_registry();
        let wildfly_image = reg.get(340).unwrap();
        assert_eq!(wildfly_image.management_port(), 9340);
    }

    #[test]
    fn platforms() {
        let reg = test_registry();
        let old = reg.get(100).unwrap();
        assert!(old.platforms.is_empty());
        let new = reg.get(390).unwrap();
        assert_eq!(new.platforms.len(), 4);
    }

    // ------------------------------------------------------ ordering

    #[test]
    fn ordering() {
        let reg = test_registry();
        let a = reg.get(100).unwrap();
        let b = reg.get(390).unwrap();
        assert!(a < b);
    }

    #[test]
    fn partial_ord_consistent_with_ord() {
        let reg = test_registry();
        let a = reg.get(100).unwrap();
        let b = reg.get(200).unwrap();
        assert_eq!(a.partial_cmp(b), Some(Ordering::Less));
        assert_eq!(b.partial_cmp(a), Some(Ordering::Greater));
        assert_eq!(a.partial_cmp(a), Some(Ordering::Equal));
    }
}
