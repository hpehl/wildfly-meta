//! Version expression parser supporting a mini-DSL for specifying WildFly images and feature packs.
//!
//! The DSL supports comma-separated items, version ranges (`20..25`), multipliers (`3x34`),
//! and mixed references to both WildFly images and feature packs (e.g. `"3x10,23..26,5x28,34,dev,ai"`).

use std::cmp::Ordering;
use std::sync::LazyLock;

use anyhow::{bail, Result};
use regex::Regex;

use crate::feature_pack::FeaturePackRegistry;
use crate::meta_item::MetaItem;
use crate::options::DslOptions;
use crate::wildfly_image::{
    identifier, wildfly_dev, WildFlyImage, WildFlyImageRegistry, DEVELOPMENT_VERSION,
};
use crate::FeaturePack;

/// Parses a single version string into a [`WildFlyImage`].
///
/// Accepts `"dev"` for the development build, a two-digit major version (e.g. `"34"`),
/// or a `major.minor` form (e.g. `"26.1"`).
static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<major>[0-9]{2})(?<dot>\.)?(?<minor>[0-9])?$").unwrap());

// ------------------------------------------------------ wildfly image

pub fn parse_wildfly_image(input: &str, registry: &WildFlyImageRegistry) -> Result<WildFlyImage> {
    if input == DEVELOPMENT_VERSION {
        Ok(wildfly_dev())
    } else {
        match VERSION_RE.captures(input) {
            Some(c) => {
                let major: u16 = c["major"].parse()?;
                let dot = c.name("dot").is_some();
                let minor_match = c.name("minor");
                if dot && minor_match.is_none() {
                    bail!("invalid version '{}'", input)
                }
                let minor: u16 = match minor_match {
                    Some(m) => m.as_str().parse()?,
                    None => 0,
                };
                match registry.get(identifier(major, minor)) {
                    Some(wildfly_image) => Ok(wildfly_image.clone()),
                    None => bail!("unknown version {}", input),
                }
            }
            None => bail!("invalid version '{}'", input),
        }
    }
}

/// Parses a comma-separated list of WildFly version expressions into a sorted list of
/// [`WildFlyImage`]s.
///
/// Supports the full mini-DSL including ranges (`20..25`), multipliers (`3x34`), and the
/// development build (`dev`). Feature pack references are rejected. The returned list is
/// sorted by identifier.
///
/// # Example
///
/// ```
/// # use wildfly_meta::*;
/// # let wildfly_images = WildFlyImageRegistry::from_toml(include_str!("../../wildfly-images.toml")).unwrap();
/// let items = parse_wildfly_images("3x10,23..26,34,dev", &wildfly_images, &DslOptions::all()).unwrap();
/// for item in &items {
///     println!("{}", item.short_name());
/// }
/// assert_eq!(items.len(), 9);
/// ```
pub fn parse_wildfly_images(
    input: &str,
    registry: &WildFlyImageRegistry,
    options: &DslOptions,
) -> Result<Vec<WildFlyImage>> {
    let mut result: Vec<WildFlyImage> = vec![];
    let mut errors: Vec<String> = vec![];

    for segment in input.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        if options.ranges && segment.contains("..") {
            match parse_range(segment, registry, options) {
                Ok(wildfly_images) => result.extend(wildfly_images),
                Err(e) => errors.push(e.to_string()),
            }
        } else if options.multipliers {
            match parse_wildfly_image_with_multiplier(segment, registry) {
                Ok(wildfly_images) => result.extend(wildfly_images),
                Err(e) => errors.push(e.to_string()),
            }
        } else {
            match parse_wildfly_image(segment, registry) {
                Ok(wildfly_image) => result.push(wildfly_image),
                Err(e) => errors.push(e.to_string()),
            }
        }
    }

    collect_results(result, errors, |wildfly_image| wildfly_image.identifier)
}

// ------------------------------------------------------ feature pack

/// Parses a feature pack reference into a [`FeaturePack`].
///
/// Accepts a bare shortcut (e.g. `"ai"`) to select the latest version, or a versioned form
/// (e.g. `"ai:0.9.0"`) to select a specific version.
pub fn parse_feature_pack(input: &str, registry: &FeaturePackRegistry) -> Result<FeaturePack> {
    if let Some((shortcut, version)) = input.split_once(':') {
        match registry.get(shortcut, version) {
            Some(feature_pack) => Ok(feature_pack.clone()),
            None => {
                let versions = registry.known_versions(shortcut);
                if versions.is_empty() {
                    bail!(
                        "Unknown feature pack '{}'. Known feature packs: {}",
                        shortcut,
                        registry.known_shortcuts().join(", ")
                    );
                } else {
                    bail!(
                        "Unknown version '{}' for feature pack '{}'. Known versions: {}",
                        version,
                        shortcut,
                        versions.join(", ")
                    );
                }
            }
        }
    } else {
        match registry.latest(input) {
            Some(feature_pack) => Ok(feature_pack.clone()),
            None => bail!(
                "Unknown feature pack '{}'. Known feature packs: {}",
                input,
                registry.known_shortcuts().join(", ")
            ),
        }
    }
}

/// Parses a comma-separated list of feature pack references into a sorted list of
/// [`FeaturePack`]s.
///
/// Supports multiplier prefixes (`3xai`) when enabled via [`DslOptions`]. Range syntax is
/// not supported for feature packs and will produce an error. WildFly version references are
/// rejected. The returned list is sorted by port offset.
///
/// # Example
///
/// ```
/// # use wildfly_meta::*;
/// # let feature_packs = FeaturePackRegistry::from_toml(include_str!("../../feature-packs.toml")).unwrap();
/// let items = parse_feature_packs("ai,grpc", &feature_packs, &DslOptions::none()).unwrap();
/// for item in &items {
///     println!("{}", item.short_name());
/// }
/// assert_eq!(items.len(), 2);
/// ```
pub fn parse_feature_packs(
    input: &str,
    registry: &FeaturePackRegistry,
    options: &DslOptions,
) -> Result<Vec<FeaturePack>> {
    let mut result: Vec<FeaturePack> = vec![];
    let mut errors: Vec<String> = vec![];

    for segment in input.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        if segment.contains("..") {
            errors.push(format!(
                "range syntax is not supported for feature packs: '{}'",
                segment
            ));
        } else if options.multipliers {
            match parse_feature_pack_with_multiplier(segment, registry) {
                Ok(items) => result.extend(items),
                Err(e) => errors.push(e.to_string()),
            }
        } else {
            match parse_feature_pack(segment, registry) {
                Ok(feature_pack) => result.push(feature_pack),
                Err(e) => errors.push(e.to_string()),
            }
        }
    }

    collect_results(result, errors, |feature_pack| feature_pack.port_offset())
}

// ------------------------------------------------------ meta item

/// Parses a single input string as either a feature pack or a WildFly image.
///
/// Feature pack lookup is tried first; if it fails, the input is parsed as a WildFly image version.
pub fn parse_meta_item(
    input: &str,
    wildfly_images: &WildFlyImageRegistry,
    feature_packs: &FeaturePackRegistry,
) -> Result<MetaItem> {
    if let Ok(feature_pack) = parse_feature_pack(input, feature_packs) {
        return Ok(MetaItem::FeaturePack(feature_pack));
    }
    parse_wildfly_image(input, wildfly_images).map(MetaItem::Image)
}

/// Parses a comma-separated list of version expressions into a sorted list of [`MetaItem`]s.
///
/// Supports the full mini-DSL including ranges (`20..25`), multipliers (`3x34`), feature pack
/// references (`ai`, `grpc:0.1.16`), and the development build (`dev`). The returned list
/// is sorted by port offset.
///
/// `wildfly_image_options` controls which DSL features are enabled for WildFly image references,
/// and `feature_pack_options` controls which DSL features are enabled for feature pack references.
/// Range syntax is only supported for WildFly images; `feature_pack_options.ranges` is ignored.
///
/// # Example
///
/// ```
/// # use wildfly_meta::*;
/// # let wildfly_images = WildFlyImageRegistry::from_toml(include_str!("../../wildfly-images.toml")).unwrap();
/// # let feature_packs = FeaturePackRegistry::from_toml(include_str!("../../feature-packs.toml")).unwrap();
/// let items = parse_meta_items("3x10,23..26,34,dev,ai", &wildfly_images, &feature_packs, &DslOptions::all(), &DslOptions::all()).unwrap();
/// for item in &items {
///     println!("{} ({})", item.short_name(), item.kind());
/// }
/// assert!(items.len() >= 10);
/// ```
pub fn parse_meta_items(
    input: &str,
    wildfly_images: &WildFlyImageRegistry,
    feature_packs: &FeaturePackRegistry,
    wildfly_image_options: &DslOptions,
    feature_pack_options: &DslOptions,
) -> Result<Vec<MetaItem>> {
    let mut result: Vec<MetaItem> = vec![];
    let mut errors: Vec<String> = vec![];

    for segment in input.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        if wildfly_image_options.ranges && segment.contains("..") {
            match parse_range(segment, wildfly_images, wildfly_image_options) {
                Ok(wildfly_images) => {
                    result.extend(wildfly_images.into_iter().map(MetaItem::Image))
                }
                Err(e) => errors.push(e.to_string()),
            }
        } else {
            match parse_meta_item_with_multiplier(
                segment,
                wildfly_images,
                feature_packs,
                wildfly_image_options,
                feature_pack_options,
            ) {
                Ok(items) => result.extend(items),
                Err(e) => errors.push(e.to_string()),
            }
        }
    }

    collect_results(result, errors, |a| a.port_offset())
}

// ------------------------------------------------------ helper functions

fn parse_range(
    range: &str,
    wildfly_images: &WildFlyImageRegistry,
    options: &DslOptions,
) -> Result<Vec<WildFlyImage>> {
    let (multiplier, range) = if options.multipliers {
        match extract_multiplier(range) {
            Some(m) => m,
            None => bail!("invalid multiplier in '{}'", range),
        }
    } else {
        (1, range)
    };

    let (start, end) = range
        .split_once("..")
        .ok_or_else(|| anyhow::anyhow!("invalid range syntax: '{}'", range))?;
    if start == DEVELOPMENT_VERSION || end == DEVELOPMENT_VERSION {
        bail!("'dev' is not allowed in range '{}'", range)
    }

    let from = match start {
        "" => wildfly_images.first().cloned(),
        _ => parse_wildfly_image(start, wildfly_images).ok(),
    };
    let to = match end {
        "" => wildfly_images.last().cloned(),
        _ => parse_wildfly_image(end, wildfly_images).ok(),
    };

    let from = from.ok_or_else(|| anyhow::anyhow!("invalid range bound: from '{}'", start))?;
    let to = to.ok_or_else(|| anyhow::anyhow!("invalid range bound: to '{}'", end))?;

    match from.identifier.cmp(&to.identifier) {
        Ordering::Equal => Ok(vec![from; multiplier as usize]),
        Ordering::Less => Ok(wildfly_images
            .range(from.identifier, to.identifier)
            .into_iter()
            .flat_map(|wildfly_image| vec![wildfly_image.clone(); multiplier as usize])
            .collect()),
        Ordering::Greater => {
            bail!("{} is greater than {}", from.identifier, to.identifier)
        }
    }
}

fn extract_multiplier(input: &str) -> Option<(u16, &str)> {
    match input.split_once('x') {
        Some((prefix, suffix)) if !suffix.is_empty() && !suffix.contains('x') => {
            let multiplier: u16 = prefix.parse().ok()?;
            if multiplier > 0 {
                Some((multiplier, suffix))
            } else {
                None
            }
        }
        Some(_) => None,
        None => Some((1, input)),
    }
}

fn parse_meta_item_with_multiplier(
    input: &str,
    wildfly_images: &WildFlyImageRegistry,
    feature_packs: &FeaturePackRegistry,
    wildfly_image_options: &DslOptions,
    feature_pack_options: &DslOptions,
) -> Result<Vec<MetaItem>> {
    let (multiplier, value) = match extract_multiplier(input) {
        Some(m) => m,
        None => bail!("invalid multiplier in '{}'", input),
    };

    if let Ok(feature_pack) = parse_feature_pack(value, feature_packs) {
        if multiplier > 1 && !feature_pack_options.multipliers {
            bail!("invalid feature pack reference '{}'", input);
        }
        Ok(vec![
            MetaItem::FeaturePack(feature_pack);
            multiplier as usize
        ])
    } else if let Ok(wildfly_image) = parse_wildfly_image(value, wildfly_images) {
        if multiplier > 1 && !wildfly_image_options.multipliers {
            bail!("invalid version '{}'", input);
        }
        Ok(vec![MetaItem::Image(wildfly_image); multiplier as usize])
    } else {
        bail!("invalid version or feature pack '{}'", value)
    }
}

fn parse_wildfly_image_with_multiplier(
    input: &str,
    registry: &WildFlyImageRegistry,
) -> Result<Vec<WildFlyImage>> {
    let (multiplier, value) = match extract_multiplier(input) {
        Some(m) => m,
        None => bail!("invalid multiplier in '{}'", input),
    };
    let wildfly_image = parse_wildfly_image(value, registry)?;
    Ok(vec![wildfly_image; multiplier as usize])
}

fn parse_feature_pack_with_multiplier(
    input: &str,
    registry: &FeaturePackRegistry,
) -> Result<Vec<FeaturePack>> {
    let (multiplier, value) = match extract_multiplier(input) {
        Some(m) => m,
        None => bail!("invalid multiplier in '{}'", input),
    };
    let feature_pack = parse_feature_pack(value, registry)?;
    Ok(vec![feature_pack; multiplier as usize])
}

fn collect_results<T, K: Ord>(
    mut result: Vec<T>,
    errors: Vec<String>,
    key: impl Fn(&T) -> K,
) -> Result<Vec<T>> {
    if errors.is_empty() {
        result.sort_by_key(|a| key(a));
        Ok(result)
    } else if errors.len() > 1 {
        bail!("\n{}", errors.join("\n"))
    } else {
        bail!("{}", errors[0])
    }
}

#[cfg(test)]
mod tests;
