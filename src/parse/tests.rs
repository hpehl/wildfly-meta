use super::*;

fn wildfly_image_registry() -> WildFlyImageRegistry {
    WildFlyImageRegistry::from_toml(include_str!("../../wildfly-images.toml")).unwrap()
}

fn feature_pack_registry() -> FeaturePackRegistry {
    FeaturePackRegistry::from_toml(include_str!("../../feature-packs.toml")).unwrap()
}

// ------------------------------------------------------ parse options

#[test]
fn parse_options_default_enables_all() {
    let opts = DslOptions::default();
    assert!(opts.ranges);
    assert!(opts.multipliers);
}

#[test]
fn parse_options_none_disables_all() {
    let opts = DslOptions::none();
    assert!(!opts.ranges);
    assert!(!opts.multipliers);
}

// ------------------------------------------------------ parse_wildfly_image

#[test]
fn parse_wildfly_image_dev() {
    let reg = wildfly_image_registry();
    let wildfly_image = parse_wildfly_image("dev", &reg).unwrap();
    assert!(wildfly_image.is_dev());
}

#[test]
fn parse_wildfly_image_major() {
    let reg = wildfly_image_registry();
    let wildfly_image = parse_wildfly_image("34", &reg).unwrap();
    assert_eq!(wildfly_image.identifier, 340);
}

#[test]
fn parse_wildfly_image_major_minor() {
    let reg = wildfly_image_registry();
    let wildfly_image = parse_wildfly_image("26.1", &reg).unwrap();
    assert_eq!(wildfly_image.identifier, 261);
}

#[test]
fn parse_wildfly_image_major_zero() {
    let reg = wildfly_image_registry();
    let wildfly_image = parse_wildfly_image("25.0", &reg).unwrap();
    assert_eq!(wildfly_image.identifier, 250);
}

#[test]
fn parse_wildfly_image_invalid() {
    let reg = wildfly_image_registry();
    assert!(parse_wildfly_image("", &reg).is_err());
    assert!(parse_wildfly_image("foo", &reg).is_err());
    assert!(parse_wildfly_image("99", &reg).is_err());
    assert!(parse_wildfly_image("10.", &reg).is_err());
    assert!(parse_wildfly_image("1.1", &reg).is_err());
}

// ------------------------------------------------------ parse_feature_pack

#[test]
fn parse_fp_shortcut() {
    let reg = feature_pack_registry();
    let feature_pack = parse_feature_pack("ai", &reg).unwrap();
    assert_eq!(feature_pack.shortcut, "ai");
    assert_eq!(feature_pack.version.to_string(), "0.9.1");
}

#[test]
fn parse_fp_versioned() {
    let reg = feature_pack_registry();
    let feature_pack = parse_feature_pack("ai:0.9.0", &reg).unwrap();
    assert_eq!(feature_pack.shortcut, "ai");
}

#[test]
fn parse_fp_unknown_shortcut() {
    let reg = feature_pack_registry();
    let result = parse_feature_pack("unknown", &reg);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Unknown feature pack"));
}

#[test]
fn parse_fp_unknown_version() {
    let reg = feature_pack_registry();
    let result = parse_feature_pack("ai:9.9.9", &reg);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Unknown version"));
}

#[test]
fn parse_fp_versioned_unknown_shortcut() {
    let reg = feature_pack_registry();
    let result = parse_feature_pack("unknown:1.0", &reg);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Unknown feature pack 'unknown'"));
}

// ------------------------------------------------------ parse_meta_item

#[test]
fn parse_meta_item_wildfly() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let item = parse_meta_item("34", &wildfly_images, &feature_packs).unwrap();
    assert!(matches!(item, MetaItem::Image(_)));
}

#[test]
fn parse_meta_item_feature_pack() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let item = parse_meta_item("ai", &wildfly_images, &feature_packs).unwrap();
    assert!(matches!(item, MetaItem::FeaturePack(_)));
}

#[test]
fn parse_meta_item_fp_takes_priority() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let item = parse_meta_item("ai", &wildfly_images, &feature_packs).unwrap();
    assert_eq!(item.kind(), "feature-pack");
}

// ------------------------------------------------------ parse_meta_items: basic

#[test]
fn parse_meta_items_single_version() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "25",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none(),
    )
    .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].short_name(), "25.0");
}

#[test]
fn parse_meta_items_multiple_versions() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "10,20,30",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none(),
    )
    .unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn parse_meta_items_feature_packs_only() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "ai,grpc",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none(),
    )
    .unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| matches!(i, MetaItem::FeaturePack(_))));
}

#[test]
fn parse_meta_items_mixed() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "34,ai,26.1",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none(),
    )
    .unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn parse_meta_items_full_dsl() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "3x10,23..26,5x28,34,dev,ai",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert!(items.len() >= 14);
}

#[test]
fn parse_meta_items_sorted_by_port_offset() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "34,10,ai",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    let offsets: Vec<u16> = items.iter().map(|i| i.port_offset()).collect();
    let mut sorted = offsets.clone();
    sorted.sort();
    assert_eq!(offsets, sorted);
}

#[test]
fn parse_meta_items_empty_segments_ignored() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        ",34,,25,",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_meta_items_whitespace_trimmed() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        " 34 , 25 ",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_meta_items_invalid() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    assert!(parse_meta_items(
        "",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none()
    )
    .is_ok());
    assert!(parse_meta_items(
        "foo",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none()
    )
    .is_err());
}

#[test]
fn parse_meta_items_multiple_errors() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let result = parse_meta_items(
        "foo,bar",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains('\n'));
}

// ------------------------------------------------------ parse_meta_items: ranges

#[test]
fn parse_meta_items_with_range() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "20..22",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 3); // 20, 21, 22
}

#[test]
fn parse_meta_items_range_from() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "30..",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert!(items.len() >= 10);
}

#[test]
fn parse_meta_items_range_to() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "..10.1",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_meta_items_range_all() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "..",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), wildfly_images.len());
}

#[test]
fn parse_meta_items_range_equal_bounds() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "25..25",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].short_name(), "25.0");
}

#[test]
fn parse_meta_items_range_reversed() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    assert!(parse_meta_items(
        "30..20",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all()
    )
    .is_err());
}

#[test]
fn parse_meta_items_range_dev_not_allowed() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    assert!(parse_meta_items(
        "dev..20",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all()
    )
    .is_err());
    assert!(parse_meta_items(
        "..dev",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all()
    )
    .is_err());
}

#[test]
fn parse_meta_items_range_without_multipliers() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let opts = DslOptions {
        ranges: true,
        multipliers: false,
    };
    let items = parse_meta_items(
        "20..22",
        &wildfly_images,
        &feature_packs,
        &opts,
        &DslOptions::none(),
    )
    .unwrap();
    assert_eq!(items.len(), 3);
}

// ------------------------------------------------------ parse_meta_items: multipliers

#[test]
fn parse_meta_items_with_multiplier() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "3x34",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|i| i.short_name() == "34.0"));
}

#[test]
fn parse_meta_items_fp_multiplier() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "2xai",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| matches!(i, MetaItem::FeaturePack(_))));
}

#[test]
fn parse_meta_items_multiplied_range() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "2x20..22",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 6); // 3 versions * 2
}

#[test]
fn parse_meta_items_invalid_multiplier_on_range() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let result = parse_meta_items(
        "0x20..25",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(result.is_err());
}

#[test]
fn parse_meta_items_invalid_multiplier_on_item() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let result = parse_meta_items(
        "0x34",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(result.is_err());
}

// ------------------------------------------------------ parse_meta_items: disabled options

#[test]
fn parse_meta_items_no_options_ignores_range_syntax() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let result = parse_meta_items(
        "20..22",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none(),
    );
    assert!(result.is_err());
}

#[test]
fn parse_meta_items_no_multiplier_treats_as_item() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let result = parse_meta_items(
        "3x34",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none(),
    );
    assert!(result.is_err());
}

// ------------------------------------------------------ parse_meta_items: mixed options

#[test]
fn parse_meta_items_image_multipliers_only() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "3x34,ai",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::none(),
    )
    .unwrap();
    assert_eq!(items.len(), 4);
}

#[test]
fn parse_meta_items_fp_multipliers_only() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let items = parse_meta_items(
        "34,2xai",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::all(),
    )
    .unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn parse_meta_items_image_multiplier_rejected_when_disabled() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let result = parse_meta_items(
        "3x34",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::all(),
    );
    assert!(result.is_err());
}

#[test]
fn parse_meta_items_fp_multiplier_rejected_when_disabled() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let result = parse_meta_items(
        "2xai",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::none(),
    );
    assert!(result.is_err());
}

// ------------------------------------------------------ meta item

#[test]
fn meta_item_short_name_feature_pack() {
    let feature_packs = feature_pack_registry();
    let feature_pack = parse_feature_pack("ai", &feature_packs).unwrap();
    let item = MetaItem::FeaturePack(feature_pack);
    assert_eq!(item.short_name(), "ai 0.9.1");
}

#[test]
fn meta_item_kind() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let wildfly_image = parse_meta_item("34", &wildfly_images, &feature_packs).unwrap();
    assert_eq!(wildfly_image.kind(), "wildfly");
    let feature_pack = parse_meta_item("ai", &wildfly_images, &feature_packs).unwrap();
    assert_eq!(feature_pack.kind(), "feature-pack");
}

#[test]
fn meta_item_expression_roundtrip() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    for input in &["34", "26.1", "ai", "grpc"] {
        let item = parse_meta_item(input, &wildfly_images, &feature_packs).unwrap();
        let reparsed =
            parse_meta_item(&item.expression(), &wildfly_images, &feature_packs).unwrap();
        assert_eq!(item, reparsed);
    }
}

#[test]
fn meta_item_full_name() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let wildfly_image = parse_meta_item("34", &wildfly_images, &feature_packs).unwrap();
    assert_eq!(wildfly_image.full_name(), "WildFly 34.0");
    let feature_pack = parse_meta_item("ai", &wildfly_images, &feature_packs).unwrap();
    assert_eq!(feature_pack.full_name(), "AI Feature Pack 0.9.1");
}

#[test]
fn meta_item_container_name() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let wildfly_image = parse_meta_item("34", &wildfly_images, &feature_packs).unwrap();
    assert_eq!(wildfly_image.container_name(), "340");
    let feature_pack = parse_meta_item("ai", &wildfly_images, &feature_packs).unwrap();
    assert_eq!(feature_pack.container_name(), "ai-0-9-1");
}

#[test]
fn meta_item_port_offset_feature_pack() {
    let feature_packs = feature_pack_registry();
    let feature_pack = parse_feature_pack("ai", &feature_packs).unwrap();
    let item = MetaItem::FeaturePack(feature_pack);
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

// ------------------------------------------------------ parse_wildfly_images

#[test]
fn parse_wildfly_images_single() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images("34", &wildfly_images, &DslOptions::none()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].short_name(), "34.0");
}

#[test]
fn parse_wildfly_images_multiple() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images("10,20,30", &wildfly_images, &DslOptions::none()).unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn parse_wildfly_images_dev() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images("dev", &wildfly_images, &DslOptions::none()).unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].is_dev());
}

#[test]
fn parse_wildfly_images_with_range() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images("20..22", &wildfly_images, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn parse_wildfly_images_with_multiplier() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images("3x34", &wildfly_images, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|i| i.short_name() == "34.0"));
}

#[test]
fn parse_wildfly_images_multiplied_range() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images("2x20..22", &wildfly_images, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 6);
}

#[test]
fn parse_wildfly_images_rejects_feature_pack() {
    let wildfly_images = wildfly_image_registry();
    assert!(parse_wildfly_images("ai", &wildfly_images, &DslOptions::none()).is_err());
}

#[test]
fn parse_wildfly_images_sorted() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images("34,10,25", &wildfly_images, &DslOptions::none()).unwrap();
    let ids: Vec<u16> = items.iter().map(|i| i.identifier).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[test]
fn parse_wildfly_images_empty_segments() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images(",34,,25,", &wildfly_images, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_wildfly_images_whitespace() {
    let wildfly_images = wildfly_image_registry();
    let items = parse_wildfly_images(" 34 , 25 ", &wildfly_images, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_wildfly_images_multiple_errors() {
    let wildfly_images = wildfly_image_registry();
    let result = parse_wildfly_images("foo,bar", &wildfly_images, &DslOptions::all());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains('\n'));
}

#[test]
fn parse_wildfly_images_disabled_options() {
    let wildfly_images = wildfly_image_registry();
    assert!(parse_wildfly_images("20..22", &wildfly_images, &DslOptions::none()).is_err());
    assert!(parse_wildfly_images("3x34", &wildfly_images, &DslOptions::none()).is_err());
}

// ------------------------------------------------------ parse_feature_packs

#[test]
fn parse_feature_packs_single() {
    let feature_packs = feature_pack_registry();
    let items = parse_feature_packs("ai", &feature_packs, &DslOptions::none()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].shortcut, "ai");
}

#[test]
fn parse_feature_packs_multiple() {
    let feature_packs = feature_pack_registry();
    let items = parse_feature_packs("ai,grpc", &feature_packs, &DslOptions::none()).unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_feature_packs_versioned() {
    let feature_packs = feature_pack_registry();
    let items = parse_feature_packs("ai:0.9.0", &feature_packs, &DslOptions::none()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].version.to_string(), "0.9.0");
}

#[test]
fn parse_feature_packs_with_multiplier() {
    let feature_packs = feature_pack_registry();
    let items = parse_feature_packs("2xai", &feature_packs, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .all(|feature_pack| feature_pack.shortcut == "ai"));
}

#[test]
fn parse_feature_packs_rejects_image_version() {
    let feature_packs = feature_pack_registry();
    assert!(parse_feature_packs("34", &feature_packs, &DslOptions::none()).is_err());
}

#[test]
fn parse_feature_packs_rejects_range() {
    let feature_packs = feature_pack_registry();
    let result = parse_feature_packs("ai..grpc", &feature_packs, &DslOptions::all());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("range syntax is not supported"));
}

#[test]
fn parse_feature_packs_sorted() {
    let feature_packs = feature_pack_registry();
    let items = parse_feature_packs("grpc,ai", &feature_packs, &DslOptions::none()).unwrap();
    let offsets: Vec<u16> = items
        .iter()
        .map(|feature_pack| feature_pack.port_offset())
        .collect();
    let mut sorted = offsets.clone();
    sorted.sort();
    assert_eq!(offsets, sorted);
}

#[test]
fn parse_feature_packs_empty_segments() {
    let feature_packs = feature_pack_registry();
    let items = parse_feature_packs(",ai,,grpc,", &feature_packs, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_feature_packs_whitespace() {
    let feature_packs = feature_pack_registry();
    let items = parse_feature_packs(" ai , grpc ", &feature_packs, &DslOptions::all()).unwrap();
    assert_eq!(items.len(), 2);
}

#[test]
fn parse_feature_packs_multiple_errors() {
    let feature_packs = feature_pack_registry();
    let result = parse_feature_packs("foo,bar", &feature_packs, &DslOptions::all());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains('\n'));
}

// ------------------------------------------------------ port offset separation

#[test]
fn port_offsets_no_overlap() {
    let wildfly_image_max = 990u16;
    let feature_pack_min = 10_000u16;
    assert!(wildfly_image_max < feature_pack_min);
}
