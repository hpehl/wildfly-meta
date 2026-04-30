//! Shell completion support for version and feature pack identifiers.

use crate::feature_pack::FeaturePackRegistry;
use crate::options::DslOptions;
use crate::parse::parse_wildfly_image;
use crate::wildfly_image::{
    identifier_major, identifier_minor, WildFlyImageRegistry, DEVELOPMENT_VERSION,
};

// ------------------------------------------------------ wildfly images

/// Returns all WildFly version identifiers for shell completion.
///
/// Includes two-digit major versions (e.g. `"34"`), `major.minor` versions (e.g. `"26.1"`),
/// and `"dev"`.
pub fn all_wildfly_images(wildfly_images: &WildFlyImageRegistry) -> Vec<String> {
    completion_versions(wildfly_images)
}

/// Returns completion suggestions for WildFly versions based on a partial input string.
///
/// Handles comma-separated lists, range expressions, and multiplier prefixes.
pub fn suggest_wildfly_images(
    input: &str,
    wildfly_images: &WildFlyImageRegistry,
    options: &DslOptions,
) -> Vec<String> {
    let candidates = completion_versions(wildfly_images);
    build_suggestions(input, candidates, Some(wildfly_images), options)
}

// ------------------------------------------------------ feature packs

/// Returns all feature pack identifiers for shell completion.
///
/// Includes bare shortcuts (e.g. `"ai"`) and versioned forms (e.g. `"ai:0.9.0"`).
pub fn all_feature_packs(feature_packs: &FeaturePackRegistry) -> Vec<String> {
    feature_packs.all_identifiers()
}

/// Returns completion suggestions for feature packs based on a partial input string.
///
/// Handles comma-separated lists and multiplier prefixes. Range completions are not
/// applicable to feature packs and are never offered.
pub fn suggest_feature_packs(
    input: &str,
    feature_packs: &FeaturePackRegistry,
    options: &DslOptions,
) -> Vec<String> {
    let candidates = feature_packs.all_identifiers();
    let feature_pack_options = DslOptions {
        ranges: false,
        multipliers: options.multipliers,
    };
    build_suggestions(input, candidates, None, &feature_pack_options)
}

// ------------------------------------------------------ meta items

/// Returns all identifiers (WildFly versions and feature packs) for shell completion.
pub fn all_meta_items(
    wildfly_images: &WildFlyImageRegistry,
    feature_packs: &FeaturePackRegistry,
) -> Vec<String> {
    let mut ids = completion_versions(wildfly_images);
    ids.extend(feature_packs.all_identifiers());
    ids
}

/// Returns completion suggestions for both WildFly versions and feature packs based on a
/// partial input string.
///
/// Handles comma-separated lists, range expressions, and multiplier prefixes.
/// `wildfly_image_options` controls which completion features are enabled for WildFly image references,
/// and `feature_pack_options` controls which completion features are enabled for feature pack references.
/// Range completions are only offered for WildFly images; `feature_pack_options.ranges` is ignored.
pub fn suggest_meta_items(
    input: &str,
    wildfly_images: &WildFlyImageRegistry,
    feature_packs: &FeaturePackRegistry,
    wildfly_image_options: &DslOptions,
    feature_pack_options: &DslOptions,
) -> Vec<String> {
    let candidates = all_meta_items(wildfly_images, feature_packs);
    let effective = DslOptions {
        ranges: wildfly_image_options.ranges,
        multipliers: wildfly_image_options.multipliers || feature_pack_options.multipliers,
    };
    build_suggestions(input, candidates, Some(wildfly_images), &effective)
}

// ------------------------------------------------------ helper functions

fn build_suggestions(
    input: &str,
    candidates: Vec<String>,
    wildfly_images: Option<&WildFlyImageRegistry>,
    options: &DslOptions,
) -> Vec<String> {
    let parameter = if input.is_empty() { None } else { Some(input) };
    let (prefix, token, suggestions) =
        find_suggestions(parameter, candidates, wildfly_images, options);
    suggestions
        .into_iter()
        .map(|s| format!("{}{}{}", prefix, token, s))
        .collect()
}

fn find_suggestions(
    parameter: Option<&str>,
    candidates: Vec<String>,
    wildfly_images: Option<&WildFlyImageRegistry>,
    options: &DslOptions,
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
                find_suggestions_inner(inner_param, &candidates, wildfly_images, options);
            return (
                prefix.to_string(),
                format!("{}{}", mult_prefix, inner_token),
                inner_suggestions,
            );
        }
    }

    let (_, out_token, suggestions) =
        find_suggestions_inner(Some(token), &candidates, wildfly_images, options);
    (prefix.to_string(), out_token, suggestions)
}

fn find_suggestions_inner(
    token: Option<&str>,
    candidates: &[String],
    wildfly_images: Option<&WildFlyImageRegistry>,
    options: &DslOptions,
) -> (String, String, Vec<String>) {
    let token = token.unwrap_or("");

    if !options.ranges || wildfly_images.is_none() {
        return ("".to_string(), "".to_string(), candidates.to_vec());
    }

    let wildfly_images = wildfly_images.unwrap();

    if token == ".." {
        let versions: Vec<String> = completion_versions(wildfly_images)
            .into_iter()
            .skip(1)
            .collect();
        (String::new(), token.to_string(), versions)
    } else if let Some(after) = token.strip_prefix("..") {
        (
            String::new(),
            token.to_string(),
            suggest_after_dots(after, 0, 0, wildfly_images),
        )
    } else if let Some(before) = token.strip_suffix("..") {
        let versions = try_parse_version(before, wildfly_images)
            .map(|(major, minor)| versions_after(major, minor, wildfly_images))
            .unwrap_or_default();
        (String::new(), token.to_string(), versions)
    } else if token.contains("..") {
        let (before, after) = token.split_once("..").unwrap_or(("", ""));
        let versions = try_parse_version(before, wildfly_images)
            .map(|(major, minor)| suggest_after_dots(after, major, minor, wildfly_images))
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

fn completion_versions(wildfly_images: &WildFlyImageRegistry) -> Vec<String> {
    let mut versions: Vec<String> = wildfly_images
        .all()
        .iter()
        .map(|wildfly_image| simple_version(wildfly_image.identifier))
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

fn try_parse_version(input: &str, wildfly_images: &WildFlyImageRegistry) -> Option<(u16, u16)> {
    parse_wildfly_image(input, wildfly_images)
        .ok()
        .map(|wildfly_image| {
            (
                identifier_major(wildfly_image.identifier),
                identifier_minor(wildfly_image.identifier),
            )
        })
}

fn versions_after(major: u16, minor: u16, wildfly_images: &WildFlyImageRegistry) -> Vec<String> {
    wildfly_images
        .all()
        .iter()
        .filter(|wildfly_image| {
            let wildfly_image_major = identifier_major(wildfly_image.identifier);
            let wildfly_image_minor = identifier_minor(wildfly_image.identifier);
            if wildfly_image_major == major {
                wildfly_image_minor > minor
            } else {
                wildfly_image_major > major
            }
        })
        .map(|wildfly_image| simple_version(wildfly_image.identifier))
        .collect()
}

fn suggest_after_dots(
    after_dots: &str,
    start_major: u16,
    start_minor: u16,
    wildfly_images: &WildFlyImageRegistry,
) -> Vec<String> {
    if parse_wildfly_image(after_dots, wildfly_images).is_ok() {
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
    wildfly_images
        .all()
        .iter()
        .filter(|wildfly_image| wildfly_image.identifier > start_id)
        .filter(|wildfly_image| {
            let wildfly_image_major = identifier_major(wildfly_image.identifier);
            let wildfly_image_minor = identifier_minor(wildfly_image.identifier);
            match number {
                1..=9 if !after_dots.ends_with('.') => {
                    wildfly_image_major >= (number * 10)
                        && wildfly_image_major < ((number + 1) * 10)
                }
                _ => wildfly_image_major == number && wildfly_image_minor > 0,
            }
        })
        .map(|wildfly_image| {
            let v = simple_version(wildfly_image.identifier);
            v.strip_prefix(after_dots).unwrap_or(&v).to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests;
