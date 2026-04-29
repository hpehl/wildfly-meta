use std::cmp::Ordering;

use anyhow::{bail, Result};
use regex::Regex;

use crate::feature_pack::FeaturePackRegistry;
use crate::image::{identifier, wildfly_dev, ImageRegistry, WildFlyImage, DEVELOPMENT_VERSION};
use crate::FeaturePack;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetaItem {
    Image(WildFlyImage),
    FeaturePack(FeaturePack),
}

impl MetaItem {
    pub fn display_name(&self) -> String {
        match self {
            MetaItem::Image(img) => img.display_version(),
            MetaItem::FeaturePack(fp) => fp.display_name(),
        }
    }

    pub fn port_offset(&self) -> u16 {
        match self {
            MetaItem::Image(img) => img.identifier,
            MetaItem::FeaturePack(fp) => fp.port_offset(),
        }
    }

    pub fn container_id(&self) -> String {
        match self {
            MetaItem::Image(img) => img.identifier.to_string(),
            MetaItem::FeaturePack(fp) => fp.container_id(),
        }
    }

    pub fn source_type(&self) -> &'static str {
        match self {
            MetaItem::Image(_) => "wildfly",
            MetaItem::FeaturePack(_) => "feature-pack",
        }
    }

    pub fn source_name(&self) -> String {
        match self {
            MetaItem::Image(img) => img.display_version(),
            MetaItem::FeaturePack(fp) => format!("{}:{}", fp.shortcut, fp.version),
        }
    }

    pub fn welcome_label(&self) -> String {
        match self {
            MetaItem::Image(img) => format!("WildFly {}", img.display_version()),
            MetaItem::FeaturePack(fp) => {
                format!("{} Feature Pack {}", fp.name, fp.version)
            }
        }
    }
}

pub struct ParseOptions {
    pub ranges: bool,
    pub multipliers: bool,
}

impl ParseOptions {
    pub fn all() -> Self {
        Self {
            ranges: true,
            multipliers: true,
        }
    }

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

pub fn parse_image(input: &str, registry: &ImageRegistry) -> Result<WildFlyImage> {
    let version_re = Regex::new(r"^(?<major>[0-9]{2})(?<dot>\.)?(?<minor>[0-9])?$").unwrap();
    if input == DEVELOPMENT_VERSION {
        Ok(wildfly_dev())
    } else {
        match version_re.captures(input) {
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

pub fn parse_item(
    input: &str,
    images: &ImageRegistry,
    packs: &FeaturePackRegistry,
) -> Result<MetaItem> {
    if let Ok(fp) = parse_feature_pack(input, packs) {
        return Ok(MetaItem::FeaturePack(fp));
    }
    parse_image(input, images).map(MetaItem::Image)
}

pub fn parse_list(
    input: &str,
    images: &ImageRegistry,
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
    images: &ImageRegistry,
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

    if !range.contains("..") {
        bail!("invalid range syntax: '{}'", range)
    }
    let parts = range.split("..").collect::<Vec<&str>>();
    if parts.len() != 2 {
        bail!("invalid range syntax: '{}'", range)
    }
    if parts[0] == DEVELOPMENT_VERSION || parts[1] == DEVELOPMENT_VERSION {
        bail!("'dev' is not allowed in range '{}'", range)
    }

    let from = match parts[0] {
        "" => images.first().cloned(),
        _ => parse_image(parts[0], images).ok(),
    };
    let to = match parts[1] {
        "" => images.last().cloned(),
        _ => parse_image(parts[1], images).ok(),
    };

    let from = from.ok_or_else(|| anyhow::anyhow!("invalid range bound: from '{}'", parts[0]))?;
    let to = to.ok_or_else(|| anyhow::anyhow!("invalid range bound: to '{}'", parts[1]))?;

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
    images: &ImageRegistry,
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
    if input.contains('x') {
        let parts = input.split('x').collect::<Vec<&str>>();
        if parts.len() == 2 && !parts[1].is_empty() {
            if let Ok(multiplier) = parts[0].parse::<u16>() {
                if multiplier > 0 {
                    return Some((multiplier, parts[1]));
                }
            }
            None
        } else {
            None
        }
    } else {
        Some((1, input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_registry() -> ImageRegistry {
        ImageRegistry::from_toml(include_str!("../wildfly-images.toml")).unwrap()
    }

    fn fp_registry() -> FeaturePackRegistry {
        FeaturePackRegistry::from_toml(include_str!("../feature-packs.toml")).unwrap()
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
        assert_eq!(item.source_type(), "feature-pack");
    }

    // ------------------------------------------------------ parse_list

    #[test]
    fn parse_list_single_version() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("25", &imgs, &fps, &ParseOptions::none()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].display_name(), "25.0");
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
    fn parse_list_with_range() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("20..22", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 3); // 20, 21, 22
    }

    #[test]
    fn parse_list_with_multiplier() {
        let imgs = image_registry();
        let fps = fp_registry();
        let items = parse_list("3x34", &imgs, &fps, &ParseOptions::all()).unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.display_name() == "34.0"));
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
        assert_eq!(items.len(), 33); // all images
    }

    #[test]
    fn parse_list_invalid() {
        let imgs = image_registry();
        let fps = fp_registry();
        assert!(parse_list("", &imgs, &fps, &ParseOptions::none()).is_ok()); // empty = empty list
        assert!(parse_list("foo", &imgs, &fps, &ParseOptions::none()).is_err());
    }

    #[test]
    fn parse_list_range_dev_not_allowed() {
        let imgs = image_registry();
        let fps = fp_registry();
        assert!(parse_list("dev..20", &imgs, &fps, &ParseOptions::all()).is_err());
        assert!(parse_list("..dev", &imgs, &fps, &ParseOptions::all()).is_err());
    }

    // ------------------------------------------------------ MetaItem

    #[test]
    fn meta_item_source_type() {
        let imgs = image_registry();
        let fps = fp_registry();
        let wf = parse_item("34", &imgs, &fps).unwrap();
        assert_eq!(wf.source_type(), "wildfly");
        let fp = parse_item("ai", &imgs, &fps).unwrap();
        assert_eq!(fp.source_type(), "feature-pack");
    }

    #[test]
    fn meta_item_source_name_roundtrip() {
        let imgs = image_registry();
        let fps = fp_registry();
        for input in &["34", "26.1", "ai", "grpc"] {
            let item = parse_item(input, &imgs, &fps).unwrap();
            let reparsed = parse_item(&item.source_name(), &imgs, &fps).unwrap();
            assert_eq!(item, reparsed);
        }
    }

    #[test]
    fn meta_item_welcome_label() {
        let imgs = image_registry();
        let fps = fp_registry();
        let wf = parse_item("34", &imgs, &fps).unwrap();
        assert_eq!(wf.welcome_label(), "WildFly 34.0");
        let fp = parse_item("ai", &imgs, &fps).unwrap();
        assert_eq!(fp.welcome_label(), "AI Feature Pack 0.9.0");
    }

    #[test]
    fn meta_item_container_id() {
        let imgs = image_registry();
        let fps = fp_registry();
        let wf = parse_item("34", &imgs, &fps).unwrap();
        assert_eq!(wf.container_id(), "340");
        let fp = parse_item("ai", &imgs, &fps).unwrap();
        assert_eq!(fp.container_id(), "ai-0-9-0");
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

    // ------------------------------------------------------ port offset separation

    #[test]
    fn port_offsets_no_overlap() {
        let wf_max = 990u16;
        let fp_min = 10_000u16;
        assert!(wf_max < fp_min);
    }
}
