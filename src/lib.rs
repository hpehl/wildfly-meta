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
//! use wildfly_meta::{update_all, ImageRegistry, FeaturePackRegistry, parse_list, ParseOptions};
//!
//! // Download or update configuration files from GitHub
//! let result = update_all().expect("failed to update");
//! println!("{}", result.summary());
//!
//! // Load registries
//! let images = ImageRegistry::load_default().expect("failed to load images");
//! let packs = FeaturePackRegistry::load_default().expect("failed to load feature packs");
//!
//! // Parse a version expression
//! let items = parse_list("34,dev,ai", &images, &packs, &ParseOptions::all()).unwrap();
//! for item in &items {
//!     println!("{}", item.display_name());
//! }
//! ```
//!
//! # Modules
//!
//! - **Images** — [`WildFlyImage`], [`ImageRegistry`], and helpers like [`wildfly_dev`] and
//!   [`identifier`].
//! - **Feature Packs** — [`FeaturePack`] and [`FeaturePackRegistry`] for Galleon feature pack
//!   metadata.
//! - **Parsing** — [`parse_list`], [`parse_image`], [`parse_feature_pack`], and [`parse_item`]
//!   for the version expression mini-DSL.
//! - **Updates** — [`update_all`] and friends for downloading TOML configuration from GitHub.
//! - **Completions** — [`suggest`] and [`all_identifiers`] for shell completion support.

mod complete;
mod feature_pack;
mod image;
mod parse;
mod update;

pub use complete::{all_identifiers, suggest, CompletionOptions};
pub use feature_pack::{FeaturePack, FeaturePackRegistry};
pub use image::{
    identifier, wildfly_dev, ImageRegistry, WildFlyImage, DEVELOPMENT_TAG, DEVELOPMENT_VERSION,
};
pub use parse::{parse_feature_pack, parse_image, parse_item, parse_list, MetaItem, ParseOptions};
pub use update::{
    config_dir, feature_packs_path, images_path, update_all, update_all_with_base_url,
    update_feature_packs, update_feature_packs_with_base_url, update_images,
    update_images_with_base_url, UpdateDiff, UpdateResult, UpdateStatus, FEATURE_PACKS_FILENAME,
    IMAGES_FILENAME,
};
