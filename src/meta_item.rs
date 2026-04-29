//! Parsed metadata item wrapping either a WildFly image or a feature pack.

use crate::feature_pack::FeaturePack;
use crate::wildfly_image::WildFlyImage;

/// A parsed metadata item — either a WildFly container image or a Galleon feature pack.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetaItem {
    /// A WildFly container image.
    Image(WildFlyImage),
    /// A Galleon feature pack.
    FeaturePack(FeaturePack),
}

impl MetaItem {
    /// Returns a short human-readable name for this item (e.g. `"34.0"` or `"ai 0.9.0"`).
    pub fn short_name(&self) -> String {
        match self {
            MetaItem::Image(img) => img.short_name(),
            MetaItem::FeaturePack(fp) => fp.short_name(),
        }
    }

    /// Returns a full branded name (e.g. `"WildFly 34.0"` or `"AI Feature Pack 0.9.0"`).
    pub fn full_name(&self) -> String {
        match self {
            MetaItem::Image(img) => img.full_name(),
            MetaItem::FeaturePack(fp) => fp.full_name(),
        }
    }

    /// Returns the port offset used to assign unique ports to this item.
    pub fn port_offset(&self) -> u16 {
        match self {
            MetaItem::Image(img) => img.identifier,
            MetaItem::FeaturePack(fp) => fp.port_offset(),
        }
    }

    /// Returns a unique container name (e.g. `"340"` or `"ai-0-9-0"`).
    pub fn container_name(&self) -> String {
        match self {
            MetaItem::Image(img) => img.identifier.to_string(),
            MetaItem::FeaturePack(fp) => fp.container_name(),
        }
    }

    /// Returns `"wildfly"` for images or `"feature-pack"` for feature packs.
    pub fn kind(&self) -> &'static str {
        match self {
            MetaItem::Image(_) => "wildfly",
            MetaItem::FeaturePack(_) => "feature-pack",
        }
    }

    /// Returns a re-parseable DSL expression (e.g. `"34.0"` or `"ai:0.9.0"`).
    pub fn expression(&self) -> String {
        match self {
            MetaItem::Image(img) => img.short_name(),
            MetaItem::FeaturePack(fp) => format!("{}:{}", fp.shortcut, fp.version),
        }
    }
}
