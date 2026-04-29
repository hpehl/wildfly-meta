//! A library for WildFly metadata: container images, feature packs, and version expression parsing.
//!
//! Data is loaded from TOML configuration files stored in `~/.config/wildfly-meta/`.
//! Use [`update_all`] to download the latest configuration from GitHub.

mod feature_pack;
mod image;
mod parse;
mod update;

pub use feature_pack::{FeaturePack, FeaturePackRegistry};
pub use image::{
    identifier, wildfly_dev, ImageRegistry, WildFlyImage, DEVELOPMENT_TAG, DEVELOPMENT_VERSION,
};
pub use parse::{parse_feature_pack, parse_image, parse_item, parse_list, MetaItem, ParseOptions};
pub use update::{
    config_dir, feature_packs_path, images_path, update_all, update_all_with_base_url,
    update_feature_packs, update_feature_packs_with_base_url, update_images,
    update_images_with_base_url, UpdateStatus, FEATURE_PACKS_FILENAME, IMAGES_FILENAME,
};
