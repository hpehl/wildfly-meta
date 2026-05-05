use std::fs;

use super::*;

fn test_registry() -> FeaturePackRegistry {
    let toml = include_str!("../../feature-packs.toml");
    FeaturePackRegistry::from_toml(toml).expect("failed to parse feature-packs.toml")
}

// ------------------------------------------------------ loading & config_version

#[test]
fn load_all_packs() {
    let reg = test_registry();
    assert!(!reg.is_empty());
}

#[test]
fn load_from_path() {
    let tmp = std::env::temp_dir().join("wildfly-meta-test-fp-load");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("feature-packs.toml");
    let content = include_str!("../../feature-packs.toml");
    fs::write(&path, content).unwrap();

    let reg = FeaturePackRegistry::load(&path, "").unwrap();
    assert!(!reg.is_empty());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn load_from_missing_path() {
    let path = Path::new("/nonexistent/feature-packs.toml");
    assert!(FeaturePackRegistry::load(path, "").is_err());
}

#[test]
fn load_missing_file_includes_resolution_hint() {
    let path = Path::new("/nonexistent/feature-packs.toml");
    let hint = "Run 'mytool update' to fix this.";
    let result = FeaturePackRegistry::load(path, hint);
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(err.contains(hint));
}

#[test]
fn load_corrupt_file_includes_resolution_hint() {
    let tmp = std::env::temp_dir().join("wildfly-meta-test-fp-corrupt");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("feature-packs.toml");
    fs::write(&path, "this is not valid toml {{{}}}").unwrap();

    let hint = "Run 'mytool update' to fix this.";
    let result = FeaturePackRegistry::load(&path, hint);
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(err.contains(hint));
    assert!(err.contains("Failed to parse"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn load_empty_resolution_hint_no_trailing_dot() {
    let path = Path::new("/nonexistent/feature-packs.toml");
    let result = FeaturePackRegistry::load(path, "");
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(!err.ends_with(". "));
}

#[test]
fn config_version_from_file() {
    let tmp = std::env::temp_dir().join("wildfly-meta-test-fp-cv");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("feature-packs.toml");
    let content = include_str!("../../feature-packs.toml");
    fs::write(&path, content).unwrap();

    let version = FeaturePackRegistry::config_version(&path).unwrap();
    assert!(version >= 1);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn config_version_missing_file() {
    let path = Path::new("/nonexistent/feature-packs.toml");
    assert!(FeaturePackRegistry::config_version(path).is_err());
}

// ------------------------------------------------------ load_or_update

#[test]
#[ignore] // requires network access
fn load_or_update_succeeds() {
    let reg = FeaturePackRegistry::load_or_update("");
    assert!(reg.is_ok());
    assert!(!reg.unwrap().is_empty());
}

// ------------------------------------------------------ registry queries

#[test]
fn get_by_shortcut_version() {
    let reg = test_registry();
    let latest = reg.latest("ai").unwrap();
    let feature_pack = reg.get("ai", &latest.version.to_string()).unwrap();
    assert_eq!(feature_pack.shortcut, "ai");
    assert_eq!(feature_pack.version, latest.version);
}

#[test]
fn get_unknown() {
    let reg = test_registry();
    assert!(reg.get("unknown", "1.0.0").is_none());
}

#[test]
fn latest_version() {
    let reg = test_registry();
    let shortcuts = reg.known_shortcuts();
    for shortcut in &shortcuts {
        let feature_pack = reg.latest(shortcut).unwrap();
        assert!(!feature_pack.version.to_string().is_empty());
        assert_eq!(feature_pack.shortcut, *shortcut);
    }
}

#[test]
fn latest_unknown() {
    let reg = test_registry();
    assert!(reg.latest("unknown").is_none());
}

#[test]
fn known_shortcuts() {
    let reg = test_registry();
    let shortcuts = reg.known_shortcuts();
    assert!(!shortcuts.is_empty());
    let mut sorted = shortcuts.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(shortcuts, sorted, "shortcuts should be sorted and unique");
}

#[test]
fn known_versions_for_shortcut() {
    let reg = test_registry();
    let versions = reg.known_versions("ai");
    assert_eq!(versions, vec!["0.8.1", "0.9.0", "0.9.1"]);
}

#[test]
fn known_versions_unknown() {
    let reg = test_registry();
    assert!(reg.known_versions("unknown").is_empty());
}

#[test]
fn all_identifiers() {
    let reg = test_registry();
    let ids = reg.all_identifiers();
    assert!(ids.contains(&"ai".to_string()));
    assert!(ids.contains(&"ai:0.9.0".to_string()));
    assert!(ids.contains(&"grpc".to_string()));
    assert!(ids.contains(&"grpc:0.1.16".to_string()));
}

#[test]
fn keys_returns_sorted() {
    let reg = test_registry();
    let keys: Vec<_> = reg.keys().collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn all_returns_all_packs() {
    let reg = test_registry();
    assert_eq!(reg.all().len(), reg.len());
}

#[test]
fn is_empty_false() {
    let reg = test_registry();
    assert!(!reg.is_empty());
}

#[test]
fn is_empty_true() {
    let toml = r#"
config_version = 1

[[feature_packs]]
shortcut = "empty"
name = "Empty"
group_id = "org.example"
artifact_id = "empty-fp"
versions = []
"#;
    let reg = FeaturePackRegistry::from_toml(toml).unwrap();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
}

// ------------------------------------------------------ index computation

#[test]
fn shortcut_index_computed() {
    let reg = test_registry();
    assert_eq!(reg.get("ai", "0.9.0").unwrap().shortcut_index, 0);
    assert_eq!(reg.get("graphql", "2.7.0").unwrap().shortcut_index, 1);
    assert_eq!(reg.get("grpc", "0.1.16").unwrap().shortcut_index, 2);
    assert_eq!(reg.get("myfaces", "2.0.3").unwrap().shortcut_index, 3);
}

#[test]
fn version_index_computed() {
    let reg = test_registry();
    assert_eq!(reg.get("ai", "0.9.1").unwrap().version_index, 0);
    assert_eq!(reg.get("ai", "0.9.0").unwrap().version_index, 1);
    assert_eq!(reg.get("ai", "0.8.1").unwrap().version_index, 2);
}

#[test]
fn multiple_versions_per_shortcut() {
    let toml = r#"
config_version = 1

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.ai"
artifact_id = "wildfly-ai-fp"
versions = [
  { version = "0.8.0", release_version = "0.8.0" },
  { version = "0.9.0", release_version = "0.9.0" },
]
"#;
    let reg = FeaturePackRegistry::from_toml(toml).unwrap();
    assert_eq!(reg.len(), 2);
    assert_eq!(reg.get("ai", "0.8.0").unwrap().version_index, 0);
    assert_eq!(reg.get("ai", "0.9.0").unwrap().version_index, 1);
    assert_eq!(reg.get("ai", "0.8.0").unwrap().shortcut_index, 0);
    assert_eq!(reg.get("ai", "0.9.0").unwrap().shortcut_index, 0);
    let latest = reg.latest("ai").unwrap();
    assert_eq!(latest.version.to_string(), "0.9.0");
    assert_eq!(reg.known_versions("ai"), vec!["0.8.0", "0.9.0"]);
}

// ------------------------------------------------------ feature pack methods

#[test]
fn port_offset() {
    let reg = test_registry();
    assert_eq!(reg.get("ai", "0.9.1").unwrap().port_offset(), 10_000);
    assert_eq!(reg.get("graphql", "2.7.0").unwrap().port_offset(), 10_100);
    assert_eq!(reg.get("grpc", "0.1.16").unwrap().port_offset(), 10_200);
    assert_eq!(reg.get("myfaces", "2.0.3").unwrap().port_offset(), 10_300);
}

#[test]
fn unique_port_offsets() {
    let reg = test_registry();
    let mut offsets: Vec<u16> = reg
        .all()
        .iter()
        .map(|feature_pack| feature_pack.port_offset())
        .collect();
    let len = offsets.len();
    offsets.sort();
    offsets.dedup();
    assert_eq!(len, offsets.len());
}

#[test]
fn port_offsets_start_at_10000() {
    let reg = test_registry();
    for feature_pack in reg.all() {
        assert!(feature_pack.port_offset() >= 10_000);
    }
}

#[test]
fn container_name() {
    let reg = test_registry();
    assert_eq!(reg.get("ai", "0.9.0").unwrap().container_name(), "ai-0-9-0");
    assert_eq!(
        reg.get("graphql", "2.7.0").unwrap().container_name(),
        "graphql-2-7-0"
    );
}

#[test]
fn download_url_without_final() {
    let reg = test_registry();
    let feature_pack = reg.get("ai", "0.9.0").unwrap();
    assert_eq!(
        feature_pack.download_url(),
        "https://repo1.maven.org/maven2/org/wildfly/generative-ai/wildfly-ai-feature-pack/0.9.0/wildfly-ai-feature-pack-0.9.0-doc.zip"
    );
}

#[test]
fn download_url_with_final() {
    let reg = test_registry();
    let feature_pack = reg.get("graphql", "2.7.0").unwrap();
    assert_eq!(
        feature_pack.download_url(),
        "https://repo1.maven.org/maven2/org/wildfly/extras/graphql/wildfly-microprofile-graphql-feature-pack/2.7.0.Final/wildfly-microprofile-graphql-feature-pack-2.7.0.Final-doc.zip"
    );
}

#[test]
fn short_name() {
    let reg = test_registry();
    assert_eq!(reg.get("ai", "0.9.0").unwrap().short_name(), "ai 0.9.0");
    assert_eq!(
        reg.get("grpc", "0.1.16").unwrap().short_name(),
        "grpc 0.1.16"
    );
}
