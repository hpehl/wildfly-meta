//! Shell completion support for version and feature pack identifiers.

use crate::feature_pack::FeaturePackRegistry;
use crate::parse::parse_wildfly_image;
use crate::wildfly_image::{
    identifier_major, identifier_minor, WildFlyImageRegistry, DEVELOPMENT_VERSION,
};

/// Controls which DSL features are enabled during completion.
pub struct CompletionOptions {
    /// Whether range expressions like `20..25` are completed.
    pub ranges: bool,
    /// Whether multiplier prefixes like `3x` are recognized during completion.
    pub multipliers: bool,
}

impl CompletionOptions {
    /// Enables all completion features (ranges and multipliers).
    pub fn all() -> Self {
        Self {
            ranges: true,
            multipliers: true,
        }
    }

    /// Disables all completion features — only plain identifiers are suggested.
    pub fn none() -> Self {
        Self {
            ranges: false,
            multipliers: false,
        }
    }
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self::all()
    }
}

/// Returns all WildFly version identifiers for shell completion.
///
/// Includes two-digit major versions (e.g. `"34"`), `major.minor` versions (e.g. `"26.1"`),
/// and `"dev"`.
pub fn all_wildfly_images(images: &WildFlyImageRegistry) -> Vec<String> {
    completion_versions(images)
}

/// Returns all feature pack identifiers for shell completion.
///
/// Includes bare shortcuts (e.g. `"ai"`) and versioned forms (e.g. `"ai:0.9.0"`).
pub fn all_feature_packs(packs: &FeaturePackRegistry) -> Vec<String> {
    packs.all_identifiers()
}

/// Returns all identifiers (WildFly versions and feature packs) for shell completion.
pub fn all_meta_items(images: &WildFlyImageRegistry, packs: &FeaturePackRegistry) -> Vec<String> {
    let mut ids = completion_versions(images);
    ids.extend(packs.all_identifiers());
    ids
}

/// Returns completion suggestions for WildFly versions based on a partial input string.
///
/// Handles comma-separated lists, range expressions, and multiplier prefixes.
pub fn suggest_wildfly_images(
    input: &str,
    images: &WildFlyImageRegistry,
    options: &CompletionOptions,
) -> Vec<String> {
    let candidates = completion_versions(images);
    build_suggestions(input, candidates, Some(images), options)
}

/// Returns completion suggestions for feature packs based on a partial input string.
///
/// Handles comma-separated lists and multiplier prefixes. Range completions are not
/// applicable to feature packs and are never offered.
pub fn suggest_feature_packs(
    input: &str,
    packs: &FeaturePackRegistry,
    options: &CompletionOptions,
) -> Vec<String> {
    let candidates = packs.all_identifiers();
    let fp_options = CompletionOptions {
        ranges: false,
        multipliers: options.multipliers,
    };
    build_suggestions(input, candidates, None, &fp_options)
}

/// Returns completion suggestions for both WildFly versions and feature packs based on a
/// partial input string.
///
/// Handles comma-separated lists, range expressions, and multiplier prefixes.
/// `image_options` controls which completion features are enabled for WildFly image references,
/// and `fp_options` controls which completion features are enabled for feature pack references.
/// Range completions are only offered for WildFly images; `fp_options.ranges` is ignored.
pub fn suggest_meta_items(
    input: &str,
    images: &WildFlyImageRegistry,
    packs: &FeaturePackRegistry,
    image_options: &CompletionOptions,
    fp_options: &CompletionOptions,
) -> Vec<String> {
    let candidates = all_meta_items(images, packs);
    let effective = CompletionOptions {
        ranges: image_options.ranges,
        multipliers: image_options.multipliers || fp_options.multipliers,
    };
    build_suggestions(input, candidates, Some(images), &effective)
}

fn build_suggestions(
    input: &str,
    candidates: Vec<String>,
    images: Option<&WildFlyImageRegistry>,
    options: &CompletionOptions,
) -> Vec<String> {
    let parameter = if input.is_empty() { None } else { Some(input) };
    let (prefix, token, suggestions) = find_suggestions(parameter, candidates, images, options);
    suggestions
        .into_iter()
        .map(|s| format!("{}{}{}", prefix, token, s))
        .collect()
}

fn find_suggestions(
    parameter: Option<&str>,
    candidates: Vec<String>,
    images: Option<&WildFlyImageRegistry>,
    options: &CompletionOptions,
) -> (String, String, Vec<String>) {
    let (prefix, token) = parse_prefix_token(parameter);

    if !options.ranges && !options.multipliers {
        return ("".to_string(), "".to_string(), candidates);
    }

    if options.multipliers {
        if let Some((mult_prefix, remainder)) = extract_completion_multiplier(token) {
            let inner_param = if remainder.is_empty() {
                None
            } else {
                Some(remainder)
            };
            let (_, inner_token, inner_suggestions) =
                find_suggestions_inner(inner_param, &candidates, images, options);
            return (
                prefix.to_string(),
                format!("{}{}", mult_prefix, inner_token),
                inner_suggestions,
            );
        }
    }

    let (_, out_token, suggestions) =
        find_suggestions_inner(Some(token), &candidates, images, options);
    (prefix.to_string(), out_token, suggestions)
}

fn find_suggestions_inner(
    token: Option<&str>,
    candidates: &[String],
    images: Option<&WildFlyImageRegistry>,
    options: &CompletionOptions,
) -> (String, String, Vec<String>) {
    let token = token.unwrap_or("");

    if !options.ranges || images.is_none() {
        return ("".to_string(), "".to_string(), candidates.to_vec());
    }

    let images = images.unwrap();

    if token == ".." {
        let versions: Vec<String> = completion_versions(images).into_iter().skip(1).collect();
        (String::new(), token.to_string(), versions)
    } else if let Some(after) = token.strip_prefix("..") {
        (
            String::new(),
            token.to_string(),
            suggest_after_dots(after, 0, 0, images),
        )
    } else if let Some(before) = token.strip_suffix("..") {
        let versions = try_parse_version(before, images)
            .map(|(major, minor)| versions_after(major, minor, images))
            .unwrap_or_default();
        (String::new(), token.to_string(), versions)
    } else if token.contains("..") {
        let (before, after) = token.split_once("..").unwrap_or(("", ""));
        let versions = try_parse_version(before, images)
            .map(|(major, minor)| suggest_after_dots(after, major, minor, images))
            .unwrap_or_default();
        (String::new(), token.to_string(), versions)
    } else {
        ("".to_string(), "".to_string(), candidates.to_vec())
    }
}

fn extract_completion_multiplier(input: &str) -> Option<(String, &str)> {
    match input.split_once('x') {
        Some((prefix, suffix)) if !suffix.contains('x') => {
            let multiplier: u16 = prefix.parse().ok()?;
            if multiplier > 1 {
                Some((format!("{}x", multiplier), suffix))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_prefix_token(parameter: Option<&str>) -> (&str, &str) {
    match parameter {
        Some(param) => match param.rfind(',') {
            Some(pos) if pos < param.len() - 1 => param.split_at(pos + 1),
            Some(_) => (param, ""),
            None => ("", param),
        },
        None => ("", ""),
    }
}

fn completion_versions(images: &WildFlyImageRegistry) -> Vec<String> {
    let mut versions: Vec<String> = images
        .all()
        .iter()
        .map(|img| simple_version(img.identifier))
        .collect();
    versions.push(DEVELOPMENT_VERSION.to_string());
    versions
}

fn simple_version(id: u16) -> String {
    let minor = identifier_minor(id);
    if minor == 0 {
        format!("{}", identifier_major(id))
    } else {
        format!("{}.{}", identifier_major(id), minor)
    }
}

fn try_parse_version(input: &str, images: &WildFlyImageRegistry) -> Option<(u16, u16)> {
    parse_wildfly_image(input, images).ok().map(|img| {
        (
            identifier_major(img.identifier),
            identifier_minor(img.identifier),
        )
    })
}

fn versions_after(major: u16, minor: u16, images: &WildFlyImageRegistry) -> Vec<String> {
    images
        .all()
        .iter()
        .filter(|img| {
            let img_major = identifier_major(img.identifier);
            let img_minor = identifier_minor(img.identifier);
            if img_major == major {
                img_minor > minor
            } else {
                img_major > major
            }
        })
        .map(|img| simple_version(img.identifier))
        .collect()
}

fn suggest_after_dots(
    after_dots: &str,
    start_major: u16,
    start_minor: u16,
    images: &WildFlyImageRegistry,
) -> Vec<String> {
    if parse_wildfly_image(after_dots, images).is_ok() {
        return vec![];
    }

    let major_number: Option<u16> = after_dots
        .strip_suffix('.')
        .unwrap_or(after_dots)
        .parse()
        .ok();

    let Some(number) = major_number else {
        return vec![];
    };

    let start_id = crate::wildfly_image::identifier(start_major, start_minor);
    images
        .all()
        .iter()
        .filter(|img| img.identifier > start_id)
        .filter(|img| {
            let img_major = identifier_major(img.identifier);
            let img_minor = identifier_minor(img.identifier);
            match number {
                1..=9 if !after_dots.ends_with('.') => {
                    img_major >= (number * 10) && img_major < ((number + 1) * 10)
                }
                _ => img_major == number && img_minor > 0,
            }
        })
        .map(|img| {
            let v = simple_version(img.identifier);
            v.strip_prefix(after_dots).unwrap_or(&v).to_string()
        })
        .collect()
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

    // ------------------------------------------------------ all_wildfly_images

    #[test]
    fn all_wildfly_images_includes_versions_and_dev() {
        let images = image_registry();
        let ids = all_wildfly_images(&images);
        assert!(ids.contains(&"34".to_string()));
        assert!(ids.contains(&"26.1".to_string()));
        assert!(ids.contains(&"dev".to_string()));
    }

    #[test]
    fn all_wildfly_images_excludes_feature_packs() {
        let images = image_registry();
        let ids = all_wildfly_images(&images);
        assert!(!ids.contains(&"ai".to_string()));
        assert!(!ids.contains(&"ai:0.9.0".to_string()));
    }

    #[test]
    fn all_wildfly_images_no_duplicates() {
        let images = image_registry();
        let ids = all_wildfly_images(&images);
        let mut deduped = ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len());
    }

    // ------------------------------------------------------ all_feature_packs

    #[test]
    fn all_feature_packs_includes_shortcuts_and_versioned() {
        let packs = fp_registry();
        let ids = all_feature_packs(&packs);
        assert!(ids.contains(&"ai".to_string()));
        assert!(ids.contains(&"ai:0.9.0".to_string()));
        assert!(ids.contains(&"grpc".to_string()));
    }

    #[test]
    fn all_feature_packs_excludes_versions() {
        let packs = fp_registry();
        let ids = all_feature_packs(&packs);
        assert!(!ids.contains(&"34".to_string()));
        assert!(!ids.contains(&"dev".to_string()));
    }

    #[test]
    fn all_feature_packs_includes_all_shortcuts() {
        let packs = fp_registry();
        let ids = all_feature_packs(&packs);
        for shortcut in &["ai", "graphql", "grpc", "keycloak", "myfaces"] {
            assert!(ids.contains(&shortcut.to_string()), "Missing: {}", shortcut);
        }
    }

    // ------------------------------------------------------ all_meta_items

    #[test]
    fn all_meta_items_includes_both() {
        let images = image_registry();
        let packs = fp_registry();
        let ids = all_meta_items(&images, &packs);
        assert!(ids.contains(&"34".to_string()));
        assert!(ids.contains(&"26.1".to_string()));
        assert!(ids.contains(&"dev".to_string()));
        assert!(ids.contains(&"ai".to_string()));
        assert!(ids.contains(&"ai:0.9.0".to_string()));
    }

    #[test]
    fn all_meta_items_no_duplicates() {
        let images = image_registry();
        let packs = fp_registry();
        let ids = all_meta_items(&images, &packs);
        let mut deduped = ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len());
    }

    // ------------------------------------------------------ suggest_wildfly_images

    #[test]
    fn suggest_wf_empty_returns_all_versions() {
        let images = image_registry();
        let results = suggest_wildfly_images("", &images, &CompletionOptions::all());
        assert!(results.contains(&"34".to_string()));
        assert!(results.contains(&"dev".to_string()));
        assert!(!results.contains(&"ai".to_string()));
    }

    #[test]
    fn suggest_wf_after_comma() {
        let images = image_registry();
        let results = suggest_wildfly_images("34,", &images, &CompletionOptions::all());
        assert!(results.iter().all(|r| r.starts_with("34,")));
        assert!(!results.is_empty());
    }

    #[test]
    fn suggest_wf_range_bare_dots() {
        let images = image_registry();
        let results = suggest_wildfly_images("..", &images, &CompletionOptions::all());
        assert!(!results.is_empty());
        let versions = completion_versions(&images);
        assert!(!results.contains(&format!("..{}", versions[0])));
    }

    #[test]
    fn suggest_wf_range_start() {
        let images = image_registry();
        let results = suggest_wildfly_images("20..", &images, &CompletionOptions::all());
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("20..")));
    }

    #[test]
    fn suggest_wf_range_dots_2() {
        let images = image_registry();
        let results = suggest_wildfly_images("..2", &images, &CompletionOptions::all());
        assert!(results.contains(&"..20".to_string()));
        assert!(results.contains(&"..26.1".to_string()));
    }

    #[test]
    fn suggest_wf_range_complete() {
        let images = image_registry();
        let results = suggest_wildfly_images("20..25", &images, &CompletionOptions::all());
        assert!(results.is_empty());
    }

    #[test]
    fn suggest_wf_range_26_dots_2() {
        let images = image_registry();
        let results = suggest_wildfly_images("26..2", &images, &CompletionOptions::all());
        assert!(results.iter().all(|r| r.starts_with("26..2")));
        assert!(results.contains(&"26..27".to_string()));
    }

    #[test]
    fn suggest_wf_range_261_dots_2() {
        let images = image_registry();
        let results = suggest_wildfly_images("26.1..2", &images, &CompletionOptions::all());
        assert!(results.iter().all(|r| r.starts_with("26.1..2")));
        assert!(results.contains(&"26.1..27".to_string()));
    }

    #[test]
    fn suggest_wf_invalid_range_start() {
        let images = image_registry();
        let results = suggest_wildfly_images("foo..", &images, &CompletionOptions::all());
        assert!(results.is_empty());
    }

    #[test]
    fn suggest_wf_invalid_range_end() {
        let images = image_registry();
        let results = suggest_wildfly_images("..foo", &images, &CompletionOptions::all());
        assert!(results.is_empty());
    }

    #[test]
    fn suggest_wf_no_options() {
        let images = image_registry();
        let results = suggest_wildfly_images("", &images, &CompletionOptions::none());
        assert!(results.contains(&"34".to_string()));
        assert!(results.contains(&"dev".to_string()));
    }

    #[test]
    fn suggest_wf_no_options_no_comma_prefix() {
        let images = image_registry();
        let results = suggest_wildfly_images("34,", &images, &CompletionOptions::none());
        assert!(!results.iter().any(|r| r.starts_with("34,")));
    }

    #[test]
    fn suggest_wf_multiplier() {
        let images = image_registry();
        let results = suggest_wildfly_images("3x", &images, &CompletionOptions::all());
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("3x")));
        assert!(results.contains(&"3x34".to_string()));
        assert!(results.contains(&"3xdev".to_string()));
    }

    #[test]
    fn suggest_wf_multiplier_with_range() {
        let images = image_registry();
        let results = suggest_wildfly_images("2x20..", &images, &CompletionOptions::all());
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("2x20..")));
    }

    #[test]
    fn suggest_wf_multiplier_disabled() {
        let images = image_registry();
        let opts = CompletionOptions {
            ranges: true,
            multipliers: false,
        };
        let results = suggest_wildfly_images("3x", &images, &opts);
        assert!(results.is_empty() || !results.iter().any(|r| r.starts_with("3x")));
    }

    #[test]
    fn suggest_wf_comma_then_range() {
        let images = image_registry();
        let results = suggest_wildfly_images("34,20..", &images, &CompletionOptions::all());
        assert!(results.iter().all(|r| r.starts_with("34,20..")));
        assert!(!results.is_empty());
    }

    // ------------------------------------------------------ suggest_feature_packs

    #[test]
    fn suggest_fp_empty_returns_all() {
        let packs = fp_registry();
        let results = suggest_feature_packs("", &packs, &CompletionOptions::all());
        assert!(results.contains(&"ai".to_string()));
        assert!(results.contains(&"grpc".to_string()));
        assert!(results.contains(&"ai:0.9.0".to_string()));
    }

    #[test]
    fn suggest_fp_excludes_versions() {
        let packs = fp_registry();
        let results = suggest_feature_packs("", &packs, &CompletionOptions::all());
        assert!(!results.contains(&"34".to_string()));
        assert!(!results.contains(&"dev".to_string()));
    }

    #[test]
    fn suggest_fp_after_comma() {
        let packs = fp_registry();
        let results = suggest_feature_packs("ai,", &packs, &CompletionOptions::all());
        assert!(results.iter().all(|r| r.starts_with("ai,")));
        assert!(!results.is_empty());
    }

    #[test]
    fn suggest_fp_multiplier() {
        let packs = fp_registry();
        let results = suggest_feature_packs("2x", &packs, &CompletionOptions::all());
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("2x")));
        assert!(results.contains(&"2xai".to_string()));
    }

    #[test]
    fn suggest_fp_no_ranges() {
        let packs = fp_registry();
        let results = suggest_feature_packs("..", &packs, &CompletionOptions::all());
        assert!(results.iter().all(|r| !r.contains("..")));
    }

    #[test]
    fn suggest_fp_no_options() {
        let packs = fp_registry();
        let results = suggest_feature_packs("", &packs, &CompletionOptions::none());
        assert!(results.contains(&"ai".to_string()));
    }

    // ------------------------------------------------------ suggest_meta_items

    #[test]
    fn suggest_meta_empty_returns_all() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(results.contains(&"34".to_string()));
        assert!(results.contains(&"ai".to_string()));
        assert!(results.contains(&"dev".to_string()));
    }

    #[test]
    fn suggest_meta_after_comma() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "34,",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(results.iter().all(|r| r.starts_with("34,")));
        assert!(results.iter().any(|r| r.ends_with("ai")));
    }

    #[test]
    fn suggest_meta_range() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "20..",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("20..")));
    }

    #[test]
    fn suggest_meta_multiplier() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "3x",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("3x")));
        assert!(results.contains(&"3x34".to_string()));
        assert!(results.contains(&"3xai".to_string()));
    }

    #[test]
    fn suggest_meta_multiplier_with_range() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "2x20..",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("2x20..")));
    }

    #[test]
    fn suggest_meta_no_options() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "",
            &images,
            &packs,
            &CompletionOptions::none(),
            &CompletionOptions::none(),
        );
        assert!(results.contains(&"34".to_string()));
        assert!(results.contains(&"ai".to_string()));
    }

    #[test]
    fn suggest_meta_comma_then_range() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "34,20..",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(results.iter().all(|r| r.starts_with("34,20..")));
        assert!(!results.is_empty());
    }

    #[test]
    fn suggest_meta_multiple_commas() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "10,26,..2",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(results.iter().all(|r| r.starts_with("10,26,")));
    }

    #[test]
    fn suggest_meta_dots_26_dot() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "..26.",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert_eq!(results, vec!["..26.1"]);
    }

    #[test]
    fn suggest_meta_dots_1000() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "..1000",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::all(),
        );
        assert!(results.is_empty());
    }

    // ------------------------------------------------------ suggest_meta_items: mixed options

    #[test]
    fn suggest_meta_mixed_ranges_for_images_only() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "20..",
            &images,
            &packs,
            &CompletionOptions::all(),
            &CompletionOptions::none(),
        );
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("20..")));
    }

    #[test]
    fn suggest_meta_no_ranges_when_image_opts_disable() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest_meta_items(
            "20..",
            &images,
            &packs,
            &CompletionOptions::none(),
            &CompletionOptions::all(),
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
        let images = image_registry();
        let versions = completion_versions(&images);
        assert!(versions.contains(&"10".to_string()));
        assert!(versions.contains(&"26.1".to_string()));
        assert!(versions.contains(&"dev".to_string()));
        assert!(!versions.contains(&"10.0".to_string()));
        assert!(!versions.contains(&"34.0".to_string()));
    }

    #[test]
    fn completion_versions_no_duplicates() {
        let images = image_registry();
        let versions = completion_versions(&images);
        let mut deduped = versions.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(versions.len(), deduped.len());
    }
}
