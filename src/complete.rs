//! Shell completion support for version and feature pack identifiers.

use crate::feature_pack::FeaturePackRegistry;
use crate::image::{identifier_major, identifier_minor, ImageRegistry, DEVELOPMENT_VERSION};
use crate::parse::parse_image;

/// Controls which kinds of completions are offered.
pub struct CompletionOptions {
    /// Whether to include feature pack shortcuts and versioned identifiers.
    pub feature_packs: bool,
    /// Whether to offer range-based completions (e.g. `20..`).
    pub ranges: bool,
}

/// Returns all valid identifiers for shell completion.
///
/// Includes WildFly version numbers, `"dev"`, and optionally feature pack shortcuts
/// and versioned identifiers (e.g. `"ai"`, `"ai:0.9.0"`).
pub fn all_identifiers(
    images: &ImageRegistry,
    packs: &FeaturePackRegistry,
    options: &CompletionOptions,
) -> Vec<String> {
    let mut ids = completion_versions(images);
    if options.feature_packs {
        ids.extend(packs.all_identifiers());
    }
    ids
}

/// Returns completion suggestions for a partial input string.
///
/// Handles comma-separated lists and range expressions, returning fully-formed completion
/// strings that include the prefix already typed by the user.
pub fn suggest(
    input: &str,
    images: &ImageRegistry,
    packs: &FeaturePackRegistry,
    options: &CompletionOptions,
) -> Vec<String> {
    let parameter = if input.is_empty() { None } else { Some(input) };
    let (prefix, token, suggestions) = find_suggestions(parameter, images, packs, options);
    suggestions
        .into_iter()
        .map(|s| format!("{}{}{}", prefix, token, s))
        .collect()
}

fn find_suggestions(
    parameter: Option<&str>,
    images: &ImageRegistry,
    packs: &FeaturePackRegistry,
    options: &CompletionOptions,
) -> (String, String, Vec<String>) {
    let (prefix, token) = parse_prefix_token(parameter);

    let (out_token, suggestions) = if !options.ranges {
        return (
            "".to_string(),
            "".to_string(),
            all_identifiers(images, packs, options),
        );
    } else if token == ".." {
        let versions: Vec<String> = completion_versions(images).into_iter().skip(1).collect();
        (token, versions)
    } else if let Some(after) = token.strip_prefix("..") {
        (token, suggest_after_dots(after, 0, 0, images))
    } else if let Some(before) = token.strip_suffix("..") {
        let versions = try_parse_version(before, images)
            .map(|(major, minor)| versions_after(major, minor, images))
            .unwrap_or_default();
        (token, versions)
    } else if token.contains("..") {
        let (before, after) = token.split_once("..").unwrap_or(("", ""));
        let versions = try_parse_version(before, images)
            .map(|(major, minor)| suggest_after_dots(after, major, minor, images))
            .unwrap_or_default();
        (token, versions)
    } else {
        ("", all_identifiers(images, packs, options))
    };

    (prefix.to_string(), out_token.to_string(), suggestions)
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

fn completion_versions(images: &ImageRegistry) -> Vec<String> {
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

fn try_parse_version(input: &str, images: &ImageRegistry) -> Option<(u16, u16)> {
    parse_image(input, images).ok().map(|img| {
        (
            identifier_major(img.identifier),
            identifier_minor(img.identifier),
        )
    })
}

fn versions_after(major: u16, minor: u16, images: &ImageRegistry) -> Vec<String> {
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
    images: &ImageRegistry,
) -> Vec<String> {
    if parse_image(after_dots, images).is_ok() {
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

    let start_id = crate::image::identifier(start_major, start_minor);
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

    fn image_registry() -> ImageRegistry {
        ImageRegistry::from_toml(include_str!("../wildfly-images.toml")).unwrap()
    }

    fn fp_registry() -> FeaturePackRegistry {
        FeaturePackRegistry::from_toml(include_str!("../feature-packs.toml")).unwrap()
    }

    fn opts_all() -> CompletionOptions {
        CompletionOptions {
            feature_packs: true,
            ranges: true,
        }
    }

    fn opts_versions_only() -> CompletionOptions {
        CompletionOptions {
            feature_packs: false,
            ranges: true,
        }
    }

    fn opts_single() -> CompletionOptions {
        CompletionOptions {
            feature_packs: true,
            ranges: false,
        }
    }

    // ------------------------------------------------------ all_identifiers

    #[test]
    fn all_identifiers_includes_versions_and_feature_packs() {
        let images = image_registry();
        let packs = fp_registry();
        let ids = all_identifiers(&images, &packs, &opts_all());
        assert!(ids.contains(&"34".to_string()));
        assert!(ids.contains(&"26.1".to_string()));
        assert!(ids.contains(&"dev".to_string()));
        assert!(ids.contains(&"ai".to_string()));
        assert!(ids.contains(&"ai:0.9.0".to_string()));
        assert!(ids.contains(&"grpc".to_string()));
    }

    #[test]
    fn all_identifiers_versions_only() {
        let images = image_registry();
        let packs = fp_registry();
        let ids = all_identifiers(&images, &packs, &opts_versions_only());
        assert!(ids.contains(&"34".to_string()));
        assert!(ids.contains(&"dev".to_string()));
        assert!(!ids.contains(&"ai".to_string()));
        assert!(!ids.contains(&"ai:0.9.0".to_string()));
    }

    #[test]
    fn all_identifiers_no_duplicates() {
        let images = image_registry();
        let packs = fp_registry();
        let ids = all_identifiers(&images, &packs, &opts_all());
        let mut deduped = ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len());
    }

    #[test]
    fn all_identifiers_includes_all_feature_pack_shortcuts() {
        let images = image_registry();
        let packs = fp_registry();
        let ids = all_identifiers(&images, &packs, &opts_all());
        for shortcut in &["ai", "graphql", "grpc", "keycloak", "myfaces"] {
            assert!(ids.contains(&shortcut.to_string()), "Missing: {}", shortcut);
        }
    }

    // ------------------------------------------------------ suggest: empty / basic

    #[test]
    fn suggest_empty_returns_all() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("", &images, &packs, &opts_all());
        assert!(results.contains(&"34".to_string()));
        assert!(results.contains(&"ai".to_string()));
        assert!(results.contains(&"dev".to_string()));
    }

    #[test]
    fn suggest_empty_versions_only() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("", &images, &packs, &opts_versions_only());
        assert!(results.contains(&"34".to_string()));
        assert!(!results.contains(&"ai".to_string()));
    }

    // ------------------------------------------------------ suggest: single (no ranges)

    #[test]
    fn suggest_single_no_ranges() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("", &images, &packs, &opts_single());
        assert!(results.contains(&"34".to_string()));
        assert!(results.contains(&"ai".to_string()));
        assert!(!results.iter().any(|r| r.contains("..")));
    }

    #[test]
    fn suggest_single_no_comma_handling() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("34,", &images, &packs, &opts_single());
        assert!(!results.iter().any(|r| r.starts_with("34,")));
    }

    // ------------------------------------------------------ suggest: comma-separated

    #[test]
    fn suggest_after_comma_returns_fresh_completions() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("34,", &images, &packs, &opts_all());
        assert!(results.iter().all(|r| r.starts_with("34,")));
        assert!(results.iter().any(|r| r.ends_with("ai")));
    }

    #[test]
    fn suggest_after_comma_with_partial() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("34,2", &images, &packs, &opts_all());
        assert!(results.iter().all(|r| r.starts_with("34,")));
    }

    #[test]
    fn suggest_comma_then_range() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("34,20..", &images, &packs, &opts_all());
        assert!(results.iter().all(|r| r.starts_with("34,20..")));
        assert!(!results.is_empty());
    }

    #[test]
    fn suggest_multiple_commas() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("10,26,..2", &images, &packs, &opts_all());
        assert!(results.iter().all(|r| r.starts_with("10,26,")));
    }

    // ------------------------------------------------------ suggest: ranges

    #[test]
    fn suggest_bare_dots_all_but_first() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("..", &images, &packs, &opts_all());
        assert!(!results.is_empty());
        let versions = completion_versions(&images);
        assert!(!results.contains(&format!("..{}", versions[0])));
    }

    #[test]
    fn suggest_dots_2() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("..2", &images, &packs, &opts_all());
        assert!(results.contains(&"..20".to_string()));
        assert!(results.contains(&"..26.1".to_string()));
    }

    #[test]
    fn suggest_dots_26_dot() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("..26.", &images, &packs, &opts_all());
        assert_eq!(results, vec!["..26.1"]);
    }

    #[test]
    fn suggest_range_start() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("20..", &images, &packs, &opts_all());
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.starts_with("20..")));
    }

    #[test]
    fn suggest_complete_range_empty() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("20..25", &images, &packs, &opts_all());
        assert!(results.is_empty());
    }

    #[test]
    fn suggest_range_26_dots_2() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("26..2", &images, &packs, &opts_all());
        assert!(results.iter().all(|r| r.starts_with("26..2")));
        assert!(results.contains(&"26..27".to_string()));
    }

    #[test]
    fn suggest_range_261_dots_2() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("26.1..2", &images, &packs, &opts_all());
        assert!(results.iter().all(|r| r.starts_with("26.1..2")));
        assert!(results.contains(&"26.1..27".to_string()));
    }

    #[test]
    fn suggest_invalid_range_start() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("foo..", &images, &packs, &opts_all());
        assert!(results.is_empty());
    }

    #[test]
    fn suggest_invalid_range_end() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("..foo", &images, &packs, &opts_all());
        assert!(results.is_empty());
    }

    #[test]
    fn suggest_dots_1000() {
        let images = image_registry();
        let packs = fp_registry();
        let results = suggest("..1000", &images, &packs, &opts_all());
        assert!(results.is_empty());
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
