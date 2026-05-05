use std::collections::BTreeSet;

use super::*;

#[test]
fn config_dir_path() {
    let dir = config_dir();
    assert!(dir.to_string_lossy().ends_with(".config/wildfly-meta"));
}

#[test]
fn wildfly_images_path_filename() {
    let path = wildfly_images_path();
    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        WILDFLY_IMAGES_FILENAME
    );
}

#[test]
fn feature_packs_path_filename() {
    let path = feature_packs_path();
    assert_eq!(
        path.file_name().unwrap().to_string_lossy(),
        FEATURE_PACKS_FILENAME
    );
}

#[test]
fn update_file_first_download() {
    let tmp = std::env::temp_dir().join("wildfly-meta-test-download");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let local = tmp.join("test.toml");
    let content = "config_version = 1\nwildfly_images = []\n";
    fs::write(&local, content).unwrap();
    let version: u32 = {
        let c = fs::read_to_string(&local).unwrap();
        let config: WildFlyImagesConfig = toml::from_str(&c).unwrap();
        config.config_version
    };
    assert_eq!(version, 1);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn summary_downloaded() {
    let status = UpdateStatus::Downloaded {
        version: 5,
        count: 33,
    };
    assert_eq!(
        status.summary("WildFly images"),
        "WildFly images downloaded (33 entries, version 5)"
    );
}

#[test]
fn summary_already_up_to_date() {
    let status = UpdateStatus::AlreadyUpToDate(3);
    assert_eq!(
        status.summary("Feature packs"),
        "Feature packs already up to date (version 3)"
    );
}

#[test]
fn summary_updated_with_diff() {
    let status = UpdateStatus::Updated {
        from_version: 5,
        to_version: 6,
        diff: UpdateDiff {
            added: vec!["WildFly 36.0".to_string(), "WildFly 35.0.1".to_string()],
            removed: vec!["WildFly 24.0".to_string()],
        },
    };
    let summary = status.summary("WildFly images");
    assert!(summary.contains("updated from version 5 to 6"));
    assert!(summary.contains("Added: WildFly 36.0, WildFly 35.0.1"));
    assert!(summary.contains("Removed: WildFly 24.0"));
}

#[test]
fn summary_updated_empty_diff() {
    let status = UpdateStatus::Updated {
        from_version: 1,
        to_version: 2,
        diff: UpdateDiff {
            added: vec![],
            removed: vec![],
        },
    };
    let summary = status.summary("WildFly images");
    assert_eq!(summary, "WildFly images updated from version 1 to 2");
}

#[test]
fn update_result_summary() {
    let result = UpdateResult {
        wildfly_images: UpdateStatus::AlreadyUpToDate(5),
        feature_packs: UpdateStatus::Downloaded {
            version: 3,
            count: 7,
        },
    };
    let summary = result.summary();
    assert!(summary.contains("WildFly images already up to date (version 5)"));
    assert!(summary.contains("Feature packs downloaded (7 entries, version 3)"));
}

#[test]
fn images_diff_computation() {
    let old_toml = r#"
config_version = 1

[[wildfly_images]]
major = 30
minor = 0
version = "30.0.0"
release_version = "30.0.0.Final"
core_version = "22.0.0"
core_release_version = "22.0.0.Final"
image_tag = "30.0.0.Final"
repository = "quay.io/wildfly/wildfly"

[[wildfly_images]]
major = 31
minor = 0
version = "31.0.0"
release_version = "31.0.0.Final"
core_version = "23.0.0"
core_release_version = "23.0.0.Final"
image_tag = "31.0.0.Final"
repository = "quay.io/wildfly/wildfly"
"#;

    let new_toml = r#"
config_version = 2

[[wildfly_images]]
major = 31
minor = 0
version = "31.0.0"
release_version = "31.0.0.Final"
core_version = "23.0.0"
core_release_version = "23.0.0.Final"
image_tag = "31.0.0.Final"
repository = "quay.io/wildfly/wildfly"

[[wildfly_images]]
major = 32
minor = 0
version = "32.0.0"
release_version = "32.0.0.Final"
core_version = "24.0.0"
core_release_version = "24.0.0.Final"
image_tag = "32.0.0.Final"
repository = "quay.io/wildfly/wildfly"
"#;

    let old_reg = WildFlyImageRegistry::from_toml(old_toml).unwrap();
    let new_reg = WildFlyImageRegistry::from_toml(new_toml).unwrap();
    let old_keys: BTreeSet<u16> = old_reg.keys().copied().collect();
    let new_keys: BTreeSet<u16> = new_reg.keys().copied().collect();

    let added: Vec<String> = new_keys
        .difference(&old_keys)
        .filter_map(|k| new_reg.get(*k))
        .map(|wildfly_image| wildfly_image.full_name())
        .collect();
    let removed: Vec<String> = old_keys
        .difference(&new_keys)
        .filter_map(|k| old_reg.get(*k))
        .map(|wildfly_image| wildfly_image.full_name())
        .collect();

    assert_eq!(added, vec!["WildFly 32.0"]);
    assert_eq!(removed, vec!["WildFly 30.0"]);
}

//noinspection DuplicatedCode
#[test]
fn feature_packs_diff_computation() {
    let old_toml = r#"
config_version = 1

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-feature-pack"
versions = [
  { version = "0.9.0", release_version = "0.9.0" },
]

[[feature_packs]]
shortcut = "graphql"
name = "GraphQL"
group_id = "org.wildfly.extras.graphql"
artifact_id = "wildfly-graphql-feature-pack"
versions = [
  { version = "2.3.0", release_version = "2.3.0.Final" },
]
"#;

    let new_toml = r#"
config_version = 2

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-feature-pack"
versions = [
  { version = "0.9.0", release_version = "0.9.0" },
  { version = "1.0.0", release_version = "1.0.0" },
]
"#;

    let old_reg = FeaturePackRegistry::from_toml(old_toml).unwrap();
    let new_reg = FeaturePackRegistry::from_toml(new_toml).unwrap();
    let old_keys: BTreeSet<&(String, semver::Version)> = old_reg.keys().collect();
    let new_keys: BTreeSet<&(String, semver::Version)> = new_reg.keys().collect();

    let added: Vec<String> = new_keys
        .difference(&old_keys)
        .filter_map(|(s, v)| new_reg.get(s, &v.to_string()))
        .map(|feature_pack| feature_pack.short_name())
        .collect();
    let removed: Vec<String> = old_keys
        .difference(&new_keys)
        .filter_map(|(s, v)| old_reg.get(s, &v.to_string()))
        .map(|feature_pack| feature_pack.short_name())
        .collect();

    assert_eq!(added, vec!["ai 1.0.0"]);
    assert_eq!(removed, vec!["graphql 2.3.0"]);
}
