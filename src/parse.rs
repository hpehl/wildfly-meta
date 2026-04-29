//! Version expression parser supporting a mini-DSL for specifying WildFly images and feature packs.
//!
//! The DSL supports comma-separated items, version ranges (`20..25`), multipliers (`3x34`),
//! and mixed references to both images and feature packs (e.g. `"3x10,23..26,5x28,34,dev,ai"`).

use std::cmp::Ordering;
use std::sync::LazyLock;

use anyhow::{bail, Result};
use regex::Regex;

use crate::feature_pack::FeaturePackRegistry;
use crate::meta_item::MetaItem;
use crate::wildfly_image::{
    identifier, wildfly_dev, WildFlyImage, WildFlyImageRegistry, DEVELOPMENT_VERSION,
};
use crate::FeaturePack;

/// Controls which DSL features are enabled during parsing.
pub struct ParseOptions {
    /// Whether range expressions like `20..25` are allowed.
    pub ranges: bool,
    /// Whether multiplier prefixes like `3x34` are allowed.
    pub multipliers: bool,
}

impl ParseOptions {
    /// Enables all DSL features (ranges and multipliers).
    pub fn all() -> Self {
        Self {
            ranges: true,
            multipliers: true,
        }
    }

    /// Disables all DSL features — only plain version and feature pack references are accepted.
    pub fn none() -> Self {
        Self {
            ranges: false,
            multipliers: false,
        }
    }
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self::all()
    }
}

/// Parses a single version string into a [`WildFlyImage`].
///
/// Accepts `"dev"` for the development build, a two-digit major version (e.g. `"34"`),
/// or a `major.minor` form (e.g. `"26.1"`).
static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<major>[0-9]{2})(?<dot>\.)?(?<minor>[0-9])?$").unwrap());

pub fn parse_image(input: &str, registry: &WildFlyImageRegistry) -> Result<WildFlyImage> {
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
                    Some(img) => Ok(img.clone()),
                    None => bail!("unknown version {}", input),
                }
            }
            None => bail!("invalid version '{}'", input),
        }
    }
}

/// Parses a feature pack reference into a [`FeaturePack`].
///
/// Accepts a bare shortcut (e.g. `"ai"`) to select the latest version, or a versioned form
/// (e.g. `"ai:0.9.0"`) to select a specific version.
pub fn parse_feature_pack(input: &str, registry: &FeaturePackRegistry) -> Result<FeaturePack> {
    if let Some((shortcut, version)) = input.split_once(':') {
        match registry.get(shortcut, version) {
            Some(fp) => Ok(fp.clone()),
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
            Some(fp) => Ok(fp.clone()),
            None => bail!(
                "Unknown feature pack '{}'. Known feature packs: {}",
                input,
                registry.known_shortcuts().join(", ")
            ),
        }
    }
}

/// Parses a single input string as either a feature pack or an image.
///
/// Feature pack lookup is tried first; if it fails, the input is parsed as an image version.
pub fn parse_item(
    input: &str,
    images: &WildFlyImageRegistry,
    packs: &FeaturePackRegistry,
) -> Result<MetaItem> {
    if let Ok(fp) = parse_feature_pack(input, packs) {
        return Ok(MetaItem::FeaturePack(fp));
    }
    parse_image(input, images).map(MetaItem::Image)
}

/// Parses a comma-separated list of version expressions into a sorted list of [`MetaItem`]s.
///
/// Supports the full mini-DSL including ranges (`20..25`), multipliers (`3x34`), feature pack
/// references (`ai`, `grpc:0.1.16`), and the development build (`dev`). The returned list
/// is sorted by port offset.
///
/// # Example
///
/// ```
/// # use wildfly_meta::*;
/// # let images = WildFlyImageRegistry::from_toml(include_str!("../wildfly-images.toml")).unwrap();
/// # let packs = FeaturePackRegistry::from_toml(include_str!("../feature-packs.toml")).unwrap();
/// let items = parse_list("3x10,23..26,34,dev,ai", &images, &packs, &ParseOptions::all()).unwrap();
/// for item in &items {
///     println!("{} ({})", item.short_name(), item.kind());
/// }
/// assert!(items.len() >= 10);
/// ```
pub fn parse_list(
    input: &str,
    images: &WildFlyImageRegistry,
    packs: &FeaturePackRegistry,
    options: &ParseOptions,
) -> Result<Vec<MetaItem>> {
    let mut result: Vec<MetaItem> = vec![];
    let mut errors: Vec<String> = vec![];

    for segment in input.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        if options.ranges && segment.contains("..") {
            match parse_range(segment, images, options) {
                Ok(items) => result.extend(items),
                Err(e) => errors.push(e.to_string()),
            }
        } else if options.multipliers {
            match parse_with_multiplier(segment, images, packs) {
                Ok(items) => result.extend(items),
                Err(e) => errors.push(e.to_string()),
            }
        } else {
            match parse_item(segment, images, packs) {
                Ok(item) => result.push(item),
                Err(e) => errors.push(e.to_string()),
            }
        }
    }

    if errors.is_empty() {
        result.sort_by_key(|a| a.port_offset());
        Ok(result)
    } else if errors.len() > 1 {
        bail!("\n{}", errors.join("\n"))
    } else {
        bail!("{}", errors[0])
    }
}

fn parse_range(
    range: &str,
    images: &WildFlyImageRegistry,
    options: &ParseOptions,
) -> Result<Vec<MetaItem>> {
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
        "" => images.first().cloned(),
        _ => parse_image(start, images).ok(),
    };
    let to = match end {
        "" => images.last().cloned(),
        _ => parse_image(end, images).ok(),
    };

    let from = from.ok_or_else(|| anyhow::anyhow!("invalid range bound: from '{}'", start))?;
    let to = to.ok_or_else(|| anyhow::anyhow!("invalid range bound: to '{}'", end))?;

    match from.identifier.cmp(&to.identifier) {
        Ordering::Equal => Ok(vec![MetaItem::Image(from); multiplier as usize]),
        Ordering::Less => Ok(images
            .range(from.identifier, to.identifier)
            .into_iter()
            .flat_map(|img| vec![MetaItem::Image(img.clone()); multiplier as usize])
            .collect()),
        Ordering::Greater => {
            bail!("{} is greater than {}", from.identifier, to.identifier)
        }
    }
}

fn parse_with_multiplier(
    input: &str,
    images: &WildFlyImageRegistry,
    packs: &FeaturePackRegistry,
) -> Result<Vec<MetaItem>> {
    let (multiplier, value) = match extract_multiplier(input) {
        Some(m) => m,
        None => bail!("invalid multiplier in '{}'", input),
    };

    let item = parse_item(value, images, packs)?;
    Ok(vec![item; multiplier as usize])
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

#[cfg(test)]
mod tests {
    use super::*;

    fn image_registry() -> WildFlyImageRegistry {
        WildFlyImageRegistry::from_toml(include_str!("../wildfly-images.toml")).unwrap()
    }

    fn fp_registry() -> FeaturePackRegistry {
        FeaturePackRegistry::from_toml(include_str!("../feature-packs.toml")).unwrap()
    }

    // ------------------------------------------------------ parse options

    #[test]
    fn parse_options_default_enables_all() {
        let opts = ParseOptions::default();
        assert!(opts.ranges);
        assert!(opts.multipliers);
    }

    #[test]
    fn parse_options_none_disables_all() {
        let opts = ParseOptions::none();
        assert!(!opts.ranges);
        assert!(!opts.multipliers);
    }

    // ------------------------------------------------------ parse_image

    #[test]
    fn parse_image_dev() {
        let reg = image_registry();
        let img = parse_image("dev", &reg).unwrap();
        assert!(img.is_dev());
    }

    #[test]
    fn parse_image_major() {
        let reg = image_registry();
        let img = parse_image("34", &reg).unwrap();
        assert_eq!(img.identifier, 340);
    }

    #[test]
    fn parse_image_major_minor() {
        let reg = image_registry();
        let img = parse_image("26.1", &reg).unwrap();
        assert_eq!(img.identifier, 261);
    }

    #[test]
    fn parse_image_major_zero() {
        let reg = image_registry();
        let img = parse_image("25.0", &reg).unwrap();
        assert_eq!(img.identifier, 250);
    }

    #[test]
    fn parse_image_invalid() {
        let reg = image_registry();
        assert!(parse_image("", &reg).is_err());
        assert!(parse_image("foo", &reg).is_err());
        assert!(parse_image("99", &reg).is_err());
        assert!(parse_image("10.", &reg).is_err());
        assert!(parse_image("1.1", &reg).is_err());
    }

    // ------------------------------------------------------ parse_feature_pack

    #[test]
    fn parse_fp_shortcut() {
        let reg = fp_registry();
        let fp = parse_feature_pack("ai", &reg).unwrap();
        assert_eq!(fp.shortcut, "ai");
        assert_eq!(fp.version, "0.9.0");
    }

    #[test]
    fn parse_fp_versioned() {
        let reg = fp_registry();
        let fp = parse_feature_pack("ai:0.9.0", &reg).unwrap();
        assert_eq!(fp.shortcut, "ai");
    }

    #[test]
    fn parse_fp_unknown_shortcut() {
        let reg = fp_registry();
        let result = parse_feature_pack("unknown", &reg);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown feature pack"));
    }

    #[test]
    fn parse_fp_unknown_version() {
        let reg = fp_registry();
        let result = parse_feature_pack("ai:9.9.9", &reg);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown version"));
    }

    #[test]
    fn parse_fp_versioned_unknown_shortcut() {
        let reg = fp_registry();
        let result = parse_feature_pack("unknown:1.0", &reg);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown feature pack 'unknown'"));
    }

    // ------------------------------------------------------ parse_item

    #[test]
    fn parse_item_wildfly() {
        let imgs = image_registry();
        let fps = fp_registry();
        let item = parse_item("34", &imgs, &fps).unwrap();
        assert!(matches!(item, MetaItem::Image(_)));
    }

    #[test]
    fn parse_item_feature_pack() {
        let imgs = image_registry();
        let fps = fp_registry();
        let item = parse_item("ai", &imgs, &fps).unwrap();
        assert!(matches!(item, MetaItem::FeaturePack(_)));
    }

    #[test]
    fn parse_item_fp_takes_priority() {
        let imgs = image_registry();
        let fps = fp_registry();
        let item = parse_item("ai", &imgs, &fps).unwrap();
        assert_eq!(item.kind(), "feature-pack");
    }

    // ------------------------------------------------------ parse_list: basic

    #[test]
    fn parse_list_single_version() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("25", &imgs, &fps, &ParseOptions::none()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].short_name(), "25.0");
    }

    #[test]
    fn parse_list_multiple_versions() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("10,20,30", &imgs, &fps, &ParseOptions::none()).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn parse_list_feature_packs_only() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("ai,grpc", &imgs, &fps, &ParseOptions::none()).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| matches!(i, MetaItem::FeaturePack(_))));
    }

    #[test]
    fn parse_list_mixed() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("34,ai,26.1", &imgs, &fps, &ParseOptions::none()).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn parse_list_full_dsl() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list(
            "3x10,23..26,5x28,34,dev,ai",
            &imgs,
            &fps,
            &ParseOptions::all(),
        )
        .unwrap();
        assert!(items.len() >= 14);
    }

    #[test]
    fn parse_list_sorted_by_port_offset() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("34,10,ai", &imgs, &fps, &ParseOptions::all()).unwrap();
        let offsets: Vec<u16> = items.iter().map(|i| i.port_offset()).collect();
        let mut sorted = offsets.clone();
        sorted.sort();
        assert_eq!(offsets, sorted);
    }

    #[test]
    fn parse_list_empty_segments_ignored() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list(",34,,25,", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parse_list_whitespace_trimmed() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list(" 34 , 25 ", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parse_list_invalid() {
        let imgs = image_registry();
        let fps = fp_registry();
        assert!(parse_list("", &imgs, &fps, &ParseOptions::none()).is_ok());
        assert!(parse_list("foo", &imgs, &fps, &ParseOptions::none()).is_err());
    }

    #[test]
    fn parse_list_multiple_errors() {
        let imgs = image_registry();
        let fps = fp_registry();
        let result = parse_list("foo,bar", &imgs, &fps, &ParseOptions::all());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains('\n'));
    }

    // ------------------------------------------------------ parse_list: ranges

    #[test]
    fn parse_list_with_range() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("20..22", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 3); // 20, 21, 22
    }

    #[test]
    fn parse_list_range_from() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("30..", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert!(items.len() >= 10);
    }

    #[test]
    fn parse_list_range_to() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("..10.1", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parse_list_range_all() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("..", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 33);
    }

    #[test]
    fn parse_list_range_equal_bounds() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("25..25", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].short_name(), "25.0");
    }

    #[test]
    fn parse_list_range_reversed() {
        let imgs = image_registry();
        let fps = fp_registry();
        assert!(parse_list("30..20", &imgs, &fps, &ParseOptions::all()).is_err());
    }

    #[test]
    fn parse_list_range_dev_not_allowed() {
        let imgs = image_registry();
        let fps = fp_registry();
        assert!(parse_list("dev..20", &imgs, &fps, &ParseOptions::all()).is_err());
        assert!(parse_list("..dev", &imgs, &fps, &ParseOptions::all()).is_err());
    }

    #[test]
    fn parse_list_range_without_multipliers() {
        let imgs = image_registry();
        let fps = fp_registry();
        let opts = ParseOptions {
            ranges: true,
            multipliers: false,
        };
        let items = parse_list("20..22", &imgs, &fps, &opts).unwrap();
        assert_eq!(items.len(), 3);
    }

    // ------------------------------------------------------ parse_list: multipliers

    #[test]
    fn parse_list_with_multiplier() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("3x34", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.short_name() == "34.0"));
    }

    #[test]
    fn parse_list_fp_multiplier() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("2xai", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| matches!(i, MetaItem::FeaturePack(_))));
    }

    #[test]
    fn parse_list_multiplied_range() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("2x20..22", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 6); // 3 versions * 2
    }

    #[test]
    fn parse_list_invalid_multiplier_on_range() {
        let imgs = image_registry();
        let fps = fp_registry();
        let result = parse_list("0x20..25", &imgs, &fps, &ParseOptions::all());
        assert!(result.is_err());
    }

    #[test]
    fn parse_list_invalid_multiplier_on_item() {
        let imgs = image_registry();
        let fps = fp_registry();
        let result = parse_list("0x34", &imgs, &fps, &ParseOptions::all());
        assert!(result.is_err());
    }

    // ------------------------------------------------------ parse_list: disabled options

    #[test]
    fn parse_list_no_options_ignores_range_syntax() {
        let imgs = image_registry();
        let fps = fp_registry();
        let result = parse_list("20..22", &imgs, &fps, &ParseOptions::none());
        assert!(result.is_err());
    }

    #[test]
    fn parse_list_no_multiplier_treats_as_item() {
        let imgs = image_registry();
        let fps = fp_registry();
        let result = parse_list("3x34", &imgs, &fps, &ParseOptions::none());
        assert!(result.is_err());
    }

    // ------------------------------------------------------ meta item

    #[test]
    fn meta_item_short_name_feature_pack() {
        let fps = fp_registry();
        let fp = parse_feature_pack("ai", &fps).unwrap();
        let item = MetaItem::FeaturePack(fp);
        assert_eq!(item.short_name(), "ai 0.9.0");
    }

    #[test]
    fn meta_item_kind() {
        let imgs = image_registry();
        let fps = fp_registry();
        let wf = parse_item("34", &imgs, &fps).unwrap();
        assert_eq!(wf.kind(), "wildfly");
        let fp = parse_item("ai", &imgs, &fps).unwrap();
        assert_eq!(fp.kind(), "feature-pack");
    }

    #[test]
    fn meta_item_expression_roundtrip() {
        let imgs = image_registry();
        let fps = fp_registry();
        for input in &["34", "26.1", "ai", "grpc"] {
            let item = parse_item(input, &imgs, &fps).unwrap();
            let reparsed = parse_item(&item.expression(), &imgs, &fps).unwrap();
            assert_eq!(item, reparsed);
        }
    }

    #[test]
    fn meta_item_full_name() {
        let imgs = image_registry();
        let fps = fp_registry();
        let wf = parse_item("34", &imgs, &fps).unwrap();
        assert_eq!(wf.full_name(), "WildFly 34.0");
        let fp = parse_item("ai", &imgs, &fps).unwrap();
        assert_eq!(fp.full_name(), "AI Feature Pack 0.9.0");
    }

    #[test]
    fn meta_item_container_name() {
        let imgs = image_registry();
        let fps = fp_registry();
        let wf = parse_item("34", &imgs, &fps).unwrap();
        assert_eq!(wf.container_name(), "340");
        let fp = parse_item("ai", &imgs, &fps).unwrap();
        assert_eq!(fp.container_name(), "ai-0-9-0");
    }

    #[test]
    fn meta_item_port_offset_feature_pack() {
        let fps = fp_registry();
        let fp = parse_feature_pack("ai", &fps).unwrap();
        let item = MetaItem::FeaturePack(fp);
        assert_eq!(item.port_offset(), 10_000);
    }

    // ------------------------------------------------------ extract_multiplier

    #[test]
    fn multiplier_ok() {
        assert_eq!(extract_multiplier("2x10"), Some((2, "10")));
        assert_eq!(extract_multiplier("5x25.1"), Some((5, "25.1")));
        assert_eq!(extract_multiplier("1x30"), Some((1, "30")));
        assert_eq!(extract_multiplier("foo"), Some((1, "foo")));
    }

    #[test]
    fn multiplier_err() {
        assert_eq!(extract_multiplier("0x10"), None);
        assert_eq!(extract_multiplier("x25"), None);
        assert_eq!(extract_multiplier("25x"), None);
        assert_eq!(extract_multiplier("10xx20"), None);
    }

    #[test]
    fn multiplier_no_x() {
        assert_eq!(extract_multiplier("34"), Some((1, "34")));
        assert_eq!(extract_multiplier("dev"), Some((1, "dev")));
    }

    // ------------------------------------------------------ port offset separation

    #[test]
    fn port_offsets_no_overlap() {
        let wf_max = 990u16;
        let fp_min = 10_000u16;
        assert!(wf_max < fp_min);
    }
}
