//! WildFly container image metadata and registry.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::registry;
use crate::update::{update_wildfly_images, wildfly_images_path};

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
        release_version: String::new(),
        core_release_version: String::new(),
        image_tag: String::new(),
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
#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize)]
pub struct WildFlyImage {
    /// Numeric identifier encoding `major * 10 + minor` (e.g. `340` for WildFly 34.0).
    pub identifier: u16,
    /// Full semantic version of the WildFly release (e.g. `34.0.1`).
    pub version: Version,
    /// Short version string for display (e.g. `"34.0"` or `"26.1"`).
    pub short_version: String,
    /// WildFly Core version bundled with this release.
    pub core_version: Version,
    /// Full WildFly release version string (e.g. `"34.0.1.Final"`).
    pub release_version: String,
    /// Full WildFly Core release version string (e.g. `"26.0.1.Final"`).
    pub core_release_version: String,
    /// Container image tag (e.g. `"34.0.1.Final-jdk21"`).
    pub image_tag: String,
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
            format!("{}:{}", self.repository, self.image_tag)
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
    pub release_version: String,
    pub core_release_version: String,
    pub image_tag: String,
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
    /// Loads the image registry from the default configuration path
    /// (`~/.config/wildfly-meta/wildfly-images.toml`).
    ///
    /// The `resolution_hint` is appended to error messages when the file is missing or
    /// unparsable, letting each consumer suggest their own recovery action
    /// (e.g. `"Run 'wado update' to fix this."`).
    pub fn load_default(resolution_hint: &str) -> Result<Self> {
        Self::load(&wildfly_images_path(), resolution_hint)
    }

    /// Loads the image registry, automatically downloading the configuration if it is missing
    /// or corrupt.
    ///
    /// If the configuration file does not exist, it is downloaded first. If loading fails
    /// (e.g. the file is corrupt or uses a deprecated format), the file is re-downloaded
    /// and loading is retried once.
    ///
    /// The `resolution_hint` is appended to error messages if the retry also fails.
    pub fn load_or_update(resolution_hint: &str) -> Result<Self> {
        registry::load_or_update(
            wildfly_images_path(),
            resolution_hint,
            update_wildfly_images,
            Self::load_default,
        )
    }

    /// Loads the image registry from the given TOML file path.
    ///
    /// The `resolution_hint` is appended to error messages when the file is missing or
    /// unparsable, letting each consumer suggest their own recovery action.
    pub fn load(path: &Path, resolution_hint: &str) -> Result<Self> {
        registry::load_toml(path, resolution_hint, Self::from_toml)
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
                release_version: entry.release_version,
                core_release_version: entry.core_release_version,
                image_tag: entry.image_tag,
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
        registry::config_version::<WildFlyImagesConfig>(path, |c| c.config_version)
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
mod tests;
