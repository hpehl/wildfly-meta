use super::*;

fn wildfly_image_registry() -> WildFlyImageRegistry {
    WildFlyImageRegistry::from_toml(include_str!("../../wildfly-images.toml")).unwrap()
}

fn feature_pack_registry() -> FeaturePackRegistry {
    FeaturePackRegistry::from_toml(include_str!("../../feature-packs.toml")).unwrap()
}

// ------------------------------------------------------ all_wildfly_images

#[test]
fn all_wildfly_images_includes_versions_and_dev() {
    let wildfly_images = wildfly_image_registry();
    let ids = all_wildfly_images(&wildfly_images);
    assert!(ids.contains(&"34".to_string()));
    assert!(ids.contains(&"26.1".to_string()));
    assert!(ids.contains(&"dev".to_string()));
}

#[test]
fn all_wildfly_images_excludes_feature_packs() {
    let wildfly_images = wildfly_image_registry();
    let ids = all_wildfly_images(&wildfly_images);
    assert!(!ids.contains(&"ai".to_string()));
    assert!(!ids.contains(&"ai:0.9.0".to_string()));
}

#[test]
fn all_wildfly_images_no_duplicates() {
    let wildfly_images = wildfly_image_registry();
    let ids = all_wildfly_images(&wildfly_images);
    let mut deduped = ids.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(ids.len(), deduped.len());
}

// ------------------------------------------------------ all_feature_packs

#[test]
fn all_feature_packs_includes_shortcuts_and_versioned() {
    let feature_packs = feature_pack_registry();
    let ids = all_feature_packs(&feature_packs);
    assert!(ids.contains(&"ai".to_string()));
    assert!(ids.contains(&"ai:0.9.0".to_string()));
    assert!(ids.contains(&"grpc".to_string()));
}

#[test]
fn all_feature_packs_excludes_versions() {
    let feature_packs = feature_pack_registry();
    let ids = all_feature_packs(&feature_packs);
    assert!(!ids.contains(&"34".to_string()));
    assert!(!ids.contains(&"dev".to_string()));
}

#[test]
fn all_feature_packs_includes_all_shortcuts() {
    let feature_packs = feature_pack_registry();
    let ids = all_feature_packs(&feature_packs);
    for shortcut in &["ai", "graphql", "grpc", "myfaces"] {
        assert!(ids.contains(&shortcut.to_string()), "Missing: {}", shortcut);
    }
}

// ------------------------------------------------------ all_meta_items

#[test]
fn all_meta_items_includes_both() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let ids = all_meta_items(&wildfly_images, &feature_packs);
    assert!(ids.contains(&"34".to_string()));
    assert!(ids.contains(&"26.1".to_string()));
    assert!(ids.contains(&"dev".to_string()));
    assert!(ids.contains(&"ai".to_string()));
    assert!(ids.contains(&"ai:0.9.0".to_string()));
}

#[test]
fn all_meta_items_no_duplicates() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let ids = all_meta_items(&wildfly_images, &feature_packs);
    let mut deduped = ids.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(ids.len(), deduped.len());
}

// ------------------------------------------------------ suggest_wildfly_images

#[test]
fn suggest_wildfly_empty_returns_all_versions() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("", &wildfly_images, &DslOptions::all());
    assert!(results.contains(&"34".to_string()));
    assert!(results.contains(&"dev".to_string()));
    assert!(!results.contains(&"ai".to_string()));
}

#[test]
fn suggest_wildfly_after_comma() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("34,", &wildfly_images, &DslOptions::all());
    assert!(results.iter().all(|r| r.starts_with("34,")));
    assert!(!results.is_empty());
}

#[test]
fn suggest_wildfly_range_bare_dots() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("..", &wildfly_images, &DslOptions::all());
    assert!(!results.is_empty());
    let versions = completion_versions(&wildfly_images);
    assert!(!results.contains(&format!("..{}", versions[0])));
}

#[test]
fn suggest_wildfly_range_start() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("20..", &wildfly_images, &DslOptions::all());
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("20..")));
}

#[test]
fn suggest_wildfly_range_dots_2() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("..2", &wildfly_images, &DslOptions::all());
    assert!(results.contains(&"..20".to_string()));
    assert!(results.contains(&"..26.1".to_string()));
}

#[test]
fn suggest_wildfly_range_complete() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("20..25", &wildfly_images, &DslOptions::all());
    assert!(results.is_empty());
}

#[test]
fn suggest_wildfly_range_26_dots_2() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("26..2", &wildfly_images, &DslOptions::all());
    assert!(results.iter().all(|r| r.starts_with("26..2")));
    assert!(results.contains(&"26..27".to_string()));
}

#[test]
fn suggest_wildfly_range_261_dots_2() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("26.1..2", &wildfly_images, &DslOptions::all());
    assert!(results.iter().all(|r| r.starts_with("26.1..2")));
    assert!(results.contains(&"26.1..27".to_string()));
}

#[test]
fn suggest_wildfly_invalid_range_start() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("foo..", &wildfly_images, &DslOptions::all());
    assert!(results.is_empty());
}

#[test]
fn suggest_wildfly_invalid_range_end() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("..foo", &wildfly_images, &DslOptions::all());
    assert!(results.is_empty());
}

#[test]
fn suggest_wildfly_no_options() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("", &wildfly_images, &DslOptions::none());
    assert!(results.contains(&"34".to_string()));
    assert!(results.contains(&"dev".to_string()));
}

#[test]
fn suggest_wildfly_no_options_no_comma_prefix() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("34,", &wildfly_images, &DslOptions::none());
    assert!(!results.iter().any(|r| r.starts_with("34,")));
}

#[test]
fn suggest_wildfly_multiplier() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("3x", &wildfly_images, &DslOptions::all());
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("3x")));
    assert!(results.contains(&"3x34".to_string()));
    assert!(results.contains(&"3xdev".to_string()));
}

#[test]
fn suggest_wildfly_multiplier_with_range() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("2x20..", &wildfly_images, &DslOptions::all());
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("2x20..")));
}

#[test]
fn suggest_wildfly_multiplier_disabled() {
    let wildfly_images = wildfly_image_registry();
    let opts = DslOptions {
        ranges: true,
        multipliers: false,
    };
    let results = suggest_wildfly_images("3x", &wildfly_images, &opts);
    assert!(results.is_empty() || !results.iter().any(|r| r.starts_with("3x")));
}

#[test]
fn suggest_wildfly_comma_then_range() {
    let wildfly_images = wildfly_image_registry();
    let results = suggest_wildfly_images("34,20..", &wildfly_images, &DslOptions::all());
    assert!(results.iter().all(|r| r.starts_with("34,20..")));
    assert!(!results.is_empty());
}

// ------------------------------------------------------ suggest_feature_packs

#[test]
fn suggest_feature_pack_empty_returns_all() {
    let feature_packs = feature_pack_registry();
    let results = suggest_feature_packs("", &feature_packs, &DslOptions::all());
    assert!(results.contains(&"ai".to_string()));
    assert!(results.contains(&"grpc".to_string()));
    assert!(results.contains(&"ai:0.9.0".to_string()));
}

#[test]
fn suggest_feature_pack_excludes_versions() {
    let feature_packs = feature_pack_registry();
    let results = suggest_feature_packs("", &feature_packs, &DslOptions::all());
    assert!(!results.contains(&"34".to_string()));
    assert!(!results.contains(&"dev".to_string()));
}

#[test]
fn suggest_feature_pack_after_comma() {
    let feature_packs = feature_pack_registry();
    let results = suggest_feature_packs("ai,", &feature_packs, &DslOptions::all());
    assert!(results.iter().all(|r| r.starts_with("ai,")));
    assert!(!results.is_empty());
}

#[test]
fn suggest_feature_pack_multiplier() {
    let feature_packs = feature_pack_registry();
    let results = suggest_feature_packs("2x", &feature_packs, &DslOptions::all());
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("2x")));
    assert!(results.contains(&"2xai".to_string()));
}

#[test]
fn suggest_feature_pack_no_ranges() {
    let feature_packs = feature_pack_registry();
    let results = suggest_feature_packs("..", &feature_packs, &DslOptions::all());
    assert!(results.iter().all(|r| !r.contains("..")));
}

#[test]
fn suggest_feature_pack_no_options() {
    let feature_packs = feature_pack_registry();
    let results = suggest_feature_packs("", &feature_packs, &DslOptions::none());
    assert!(results.contains(&"ai".to_string()));
}

// ------------------------------------------------------ suggest_meta_items

#[test]
fn suggest_meta_empty_returns_all() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(results.contains(&"34".to_string()));
    assert!(results.contains(&"ai".to_string()));
    assert!(results.contains(&"dev".to_string()));
}

#[test]
fn suggest_meta_after_comma() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "34,",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(results.iter().all(|r| r.starts_with("34,")));
    assert!(results.iter().any(|r| r.ends_with("ai")));
}

#[test]
fn suggest_meta_range() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "20..",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("20..")));
}

#[test]
fn suggest_meta_multiplier() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "3x",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("3x")));
    assert!(results.contains(&"3x34".to_string()));
    assert!(results.contains(&"3xai".to_string()));
}

#[test]
fn suggest_meta_multiplier_with_range() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "2x20..",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("2x20..")));
}

#[test]
fn suggest_meta_no_options() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::none(),
    );
    assert!(results.contains(&"34".to_string()));
    assert!(results.contains(&"ai".to_string()));
}

#[test]
fn suggest_meta_comma_then_range() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "34,20..",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(results.iter().all(|r| r.starts_with("34,20..")));
    assert!(!results.is_empty());
}

#[test]
fn suggest_meta_multiple_commas() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "10,26,..2",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(results.iter().all(|r| r.starts_with("10,26,")));
}

#[test]
fn suggest_meta_dots_26_dot() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "..26.",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert_eq!(results, vec!["..26.1"]);
}

#[test]
fn suggest_meta_dots_1000() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "..1000",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::all(),
    );
    assert!(results.is_empty());
}

// ------------------------------------------------------ suggest_meta_items: mixed options

#[test]
fn suggest_meta_mixed_ranges_for_images_only() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "20..",
        &wildfly_images,
        &feature_packs,
        &DslOptions::all(),
        &DslOptions::none(),
    );
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.starts_with("20..")));
}

#[test]
fn suggest_meta_no_ranges_when_image_opts_disable() {
    let wildfly_images = wildfly_image_registry();
    let feature_packs = feature_pack_registry();
    let results = suggest_meta_items(
        "20..",
        &wildfly_images,
        &feature_packs,
        &DslOptions::none(),
        &DslOptions::all(),
    );
    assert!(results.iter().all(|r| !r.contains("..")));
}

// ------------------------------------------------------ parse_prefix_token

#[test]
fn prefix_token_none() {
    let (prefix, token) = parse_prefix_token(None);
    assert_eq!(prefix, "");
    assert_eq!(token, "");
}

#[test]
fn prefix_token_simple() {
    let (prefix, token) = parse_prefix_token(Some("34"));
    assert_eq!(prefix, "");
    assert_eq!(token, "34");
}

#[test]
fn prefix_token_after_comma() {
    let (prefix, token) = parse_prefix_token(Some("34,26"));
    assert_eq!(prefix, "34,");
    assert_eq!(token, "26");
}

#[test]
fn prefix_token_trailing_comma() {
    let (prefix, token) = parse_prefix_token(Some("34,"));
    assert_eq!(prefix, "34,");
    assert_eq!(token, "");
}

// ------------------------------------------------------ completion_versions

#[test]
fn completion_versions_format() {
    let wildfly_images = wildfly_image_registry();
    let versions = completion_versions(&wildfly_images);
    assert!(versions.contains(&"10".to_string()));
    assert!(versions.contains(&"26.1".to_string()));
    assert!(versions.contains(&"dev".to_string()));
    assert!(!versions.contains(&"10.0".to_string()));
    assert!(!versions.contains(&"34.0".to_string()));
}

#[test]
fn completion_versions_no_duplicates() {
    let wildfly_images = wildfly_image_registry();
    let versions = completion_versions(&wildfly_images);
    let mut deduped = versions.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(versions.len(), deduped.len());
}
