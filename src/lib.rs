//! A library for managing WildFly metadata: container images, feature packs, and version
//! expression parsing.
//!
//! # Overview
//!
//! This crate provides registries for WildFly container images and feature packs, loaded from
//! TOML configuration files stored in `~/.config/wildfly-meta/`. It also includes a mini-DSL
//! parser for specifying versions and feature packs in a compact notation.
//!
//! # Quick Start
//!
//! ```no_run
//! use wildfly_meta::{update_all, WildFlyImageRegistry, FeaturePackRegistry, parse_meta_items, ParseOptions};
//!
//! // Download or update configuration files from GitHub
//! let result = update_all().expect("failed to update");
//! println!("{}", result.summary());
//!
//! // Load registries
//! let wildfly_images = WildFlyImageRegistry::load_default().expect("failed to load images");
//! let feature_packs = FeaturePackRegistry::load_default().expect("failed to load feature packs");
//!
//! // Parse a version expression
//! let items = parse_meta_items("34,dev,ai", &wildfly_images, &feature_packs, &ParseOptions::all(), &ParseOptions::all()).unwrap();
//! for item in &items {
//!     println!("{}", item.short_name());
//! }
//! ```
//!
//! # Modules
//!
//! - **Images** — [`WildFlyImage`], [`WildFlyImageRegistry`], and helpers like [`wildfly_dev`] and
//!   [`identifier`].
//! - **Feature Packs** — [`FeaturePack`] and [`FeaturePackRegistry`] for Galleon feature pack
//!   metadata.
//! - **Parsing** — [`parse_meta_items`], [`parse_wildfly_image`], [`parse_wildfly_images`],
//!   [`parse_feature_pack`], [`parse_feature_packs`], and [`parse_meta_item`] for the version
//!   expression mini-DSL.
//! - **Updates** — [`update_all`] and friends for downloading TOML configuration from GitHub.
//! - **Completions** — [`suggest_wildfly_images`], [`suggest_feature_packs`],
//!   [`suggest_meta_items`], [`all_wildfly_images`], [`all_feature_packs`], and
//!   [`all_meta_items`] for shell completion support.

mod complete;
mod feature_pack;
mod meta_item;
mod parse;
mod update;
mod wildfly_image;

pub use complete::{
    all_feature_packs, all_meta_items, all_wildfly_images, suggest_feature_packs,
    suggest_meta_items, suggest_wildfly_images, CompletionOptions,
};
pub use feature_pack::{FeaturePack, FeaturePackRegistry};
pub use meta_item::MetaItem;
pub use parse::{
    parse_feature_pack, parse_feature_packs, parse_meta_item, parse_meta_items,
    parse_wildfly_image, parse_wildfly_images, ParseOptions,
};
pub use update::{
    config_dir, feature_packs_path, update_all, update_all_with_base_url, update_feature_packs,
    update_feature_packs_with_base_url, update_wildfly_images, update_wildfly_images_with_base_url,
    wildfly_images_path, UpdateDiff, UpdateResult, UpdateStatus, FEATURE_PACKS_FILENAME,
    WILDFLY_IMAGES_FILENAME,
};
pub use wildfly_image::{
    identifier, identifier_major, identifier_minor, wildfly_dev, WildFlyImage,
    WildFlyImageRegistry, DEVELOPMENT_TAG, DEVELOPMENT_VERSION,
};
