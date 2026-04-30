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
            MetaItem::Image(wildfly_image) => wildfly_image.short_name(),
            MetaItem::FeaturePack(feature_pack) => feature_pack.short_name(),
        }
    }

    /// Returns a full branded name (e.g. `"WildFly 34.0"` or `"AI Feature Pack 0.9.0"`).
    pub fn full_name(&self) -> String {
        match self {
            MetaItem::Image(wildfly_image) => wildfly_image.full_name(),
            MetaItem::FeaturePack(feature_pack) => feature_pack.full_name(),
        }
    }

    /// Returns the port offset used to assign unique ports to this item.
    pub fn port_offset(&self) -> u16 {
        match self {
            MetaItem::Image(wildfly_image) => wildfly_image.identifier,
            MetaItem::FeaturePack(feature_pack) => feature_pack.port_offset(),
        }
    }

    /// Returns a unique container name (e.g. `"340"` or `"ai-0-9-0"`).
    pub fn container_name(&self) -> String {
        match self {
            MetaItem::Image(wildfly_image) => wildfly_image.identifier.to_string(),
            MetaItem::FeaturePack(feature_pack) => feature_pack.container_name(),
        }
    }

    /// Returns `"wildfly"` for WildFly images or `"feature-pack"` for feature packs.
    pub fn kind(&self) -> &'static str {
        match self {
            MetaItem::Image(_) => "wildfly",
            MetaItem::FeaturePack(_) => "feature-pack",
        }
    }

    /// Returns a re-parseable DSL expression (e.g. `"34.0"` or `"ai:0.9.0"`).
    pub fn expression(&self) -> String {
        match self {
            MetaItem::Image(wildfly_image) => wildfly_image.short_name(),
            MetaItem::FeaturePack(feature_pack) => {
                format!("{}:{}", feature_pack.shortcut, feature_pack.version)
            }
        }
    }
}
