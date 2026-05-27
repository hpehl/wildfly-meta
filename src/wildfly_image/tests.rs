use std::cmp::Ordering;
use std::fs;

use super::*;

fn test_registry() -> WildFlyImageRegistry {
    let toml = include_str!("../../wildfly-images.toml");
    WildFlyImageRegistry::from_toml(toml).expect("failed to parse wildfly-images.toml")
}

// ------------------------------------------------------ loading & config_version

#[test]
fn load_all_images() {
    let reg = test_registry();
    assert!(reg.len() > 0);
    assert_eq!(reg.all().len(), reg.len());
}

#[test]
fn load_from_path() {
    let tmp = std::env::temp_dir().join("wildfly-meta-test-wildfly-image-load");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("wildfly-images.toml");
    let content = include_str!("../../wildfly-images.toml");
    fs::write(&path, content).unwrap();

    let reg = WildFlyImageRegistry::load(&path, "").unwrap();
    let expected = test_registry();
    assert_eq!(reg.len(), expected.len());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn load_from_missing_path() {
    let path = Path::new("/nonexistent/wildfly-images.toml");
    assert!(WildFlyImageRegistry::load(path, "").is_err());
}

#[test]
fn load_missing_file_includes_resolution_hint() {
    let path = Path::new("/nonexistent/wildfly-images.toml");
    let hint = "Run 'mytool update' to fix this.";
    let result = WildFlyImageRegistry::load(path, hint);
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(err.contains(hint));
}

#[test]
fn load_corrupt_file_includes_resolution_hint() {
    let tmp = std::env::temp_dir().join("wildfly-meta-test-wildfly-image-corrupt");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("wildfly-images.toml");
    fs::write(&path, "this is not valid toml {{{}}}").unwrap();

    let hint = "Run 'mytool update' to fix this.";
    let result = WildFlyImageRegistry::load(&path, hint);
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(err.contains(hint));
    assert!(err.contains("Failed to parse"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn load_empty_resolution_hint_no_trailing_dot() {
    let path = Path::new("/nonexistent/wildfly-images.toml");
    let result = WildFlyImageRegistry::load(path, "");
    assert!(result.is_err());
    let err = format!("{}", result.err().unwrap());
    assert!(!err.ends_with(". "));
}

#[test]
fn config_version_from_file() {
    let tmp = std::env::temp_dir().join("wildfly-meta-test-wildfly-image-cv");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let path = tmp.join("wildfly-images.toml");
    let content = include_str!("../../wildfly-images.toml");
    fs::write(&path, content).unwrap();

    let version = WildFlyImageRegistry::config_version(&path).unwrap();
    assert!(version >= 1);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn config_version_missing_file() {
    let path = Path::new("/nonexistent/wildfly-images.toml");
    assert!(WildFlyImageRegistry::config_version(path).is_err());
}

// ------------------------------------------------------ load_or_update

#[test]
#[ignore] // requires network access
fn load_or_update_succeeds() {
    let reg = WildFlyImageRegistry::load_or_update("");
    assert!(reg.is_ok());
    assert!(!reg.unwrap().is_empty());
}

#[test]
#[ignore] // requires network access
fn load_or_update_includes_resolution_hint_on_failure() {
    let hint = "Run 'mytool update' to fix this.";
    let reg = WildFlyImageRegistry::load_or_update(hint);
    assert!(reg.is_ok());
}

// ------------------------------------------------------ registry queries

#[test]
fn get_by_identifier() {
    let reg = test_registry();
    let wildfly_image = reg.get(261).unwrap();
    assert_eq!(wildfly_image.short_version, "26.1");
    assert_eq!(wildfly_image.release_version, "26.1.3.Final");
    assert_eq!(wildfly_image.core_release_version, "18.1.2.Final");
    assert_eq!(wildfly_image.image_tag, "26.1.3.Final-jdk17");
}

#[test]
fn get_unknown() {
    let reg = test_registry();
    assert!(reg.get(999).is_none());
}

#[test]
fn first_image() {
    let reg = test_registry();
    let first = reg.first().unwrap();
    assert_eq!(first.identifier, 100);
    assert_eq!(first.short_version, "10.0");
}

#[test]
fn last_image() {
    let reg = test_registry();
    let first = reg.first().unwrap();
    let last = reg.last().unwrap();
    assert!(last.identifier > first.identifier);
    assert!(!last.short_version.is_empty());
}

#[test]
fn first_and_last_on_empty() {
    let toml = r#"
config_version = 1
wildfly_images = []
"#;
    let reg = WildFlyImageRegistry::from_toml(toml).unwrap();
    assert!(reg.first().is_none());
    assert!(reg.last().is_none());
}

#[test]
fn range_query() {
    let reg = test_registry();
    let wildfly_images = reg.range(200, 220);
    assert_eq!(wildfly_images.len(), 3); // 20.0, 21.0, 22.0
}

#[test]
fn range_empty_result() {
    let reg = test_registry();
    let wildfly_images = reg.range(500, 600);
    assert!(wildfly_images.is_empty());
}

#[test]
fn keys_returns_sorted() {
    let reg = test_registry();
    let keys: Vec<_> = reg.keys().copied().collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn all_returns_all_images() {
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
wildfly_images = []
"#;
    let reg = WildFlyImageRegistry::from_toml(toml).unwrap();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
}

// ------------------------------------------------------ identifier helpers

#[test]
fn identifier_roundtrip() {
    assert_eq!(identifier(34, 0), 340);
    assert_eq!(identifier_major(340), 34);
    assert_eq!(identifier_minor(340), 0);
    assert_eq!(identifier(26, 1), 261);
    assert_eq!(identifier_major(261), 26);
    assert_eq!(identifier_minor(261), 1);
}

// ------------------------------------------------------ wildfly_dev

#[test]
fn wildfly_dev_fields() {
    let dev = wildfly_dev();
    assert_eq!(dev.identifier, 0);
    assert!(dev.is_dev());
    assert_eq!(dev.http_port(), 8000);
    assert_eq!(dev.management_port(), 9000);
    assert!(dev.short_version.is_empty());
    assert!(dev.release_version.is_empty());
    assert!(dev.core_release_version.is_empty());
    assert!(dev.image_tag.is_empty());
    assert!(dev.platforms.is_empty());
}

#[test]
fn is_dev() {
    let dev = wildfly_dev();
    assert!(dev.is_dev());
    let reg = test_registry();
    assert!(!reg.get(100).unwrap().is_dev());
}

// ------------------------------------------------------ wildfly image methods

#[test]
fn short_name_regular() {
    let reg = test_registry();
    let wildfly_image = reg.get(250).unwrap();
    assert_eq!(wildfly_image.short_name(), "25.0");
    let wildfly_image = reg.get(261).unwrap();
    assert_eq!(wildfly_image.short_name(), "26.1");
}

#[test]
fn short_name_dev() {
    let dev = wildfly_dev();
    assert_eq!(dev.short_name(), "dev");
}

#[test]
fn full_name_regular() {
    let reg = test_registry();
    let wildfly_image = reg.get(250).unwrap();
    assert_eq!(wildfly_image.full_name(), "WildFly 25.0");
    let wildfly_image = reg.get(261).unwrap();
    assert_eq!(wildfly_image.full_name(), "WildFly 26.1");
}

#[test]
fn full_name_dev() {
    let dev = wildfly_dev();
    assert_eq!(dev.full_name(), "WildFly dev");
}

#[test]
fn image_ref_regular() {
    let reg = test_registry();
    let wildfly_image = reg.get(390).unwrap();
    assert!(wildfly_image
        .image_ref()
        .starts_with("quay.io/wildfly/wildfly:"));
}

#[test]
fn image_ref_dev() {
    let dev = wildfly_dev();
    assert_eq!(dev.image_ref(), "https://github.com/wildfly/wildfly.git");
}

#[test]
fn image_ref_uses_image_tag() {
    let reg = test_registry();
    let img = reg.get(261).unwrap();
    let name = img.image_ref();
    assert!(name.contains("26.1.3.Final-jdk17"));
    assert_eq!(name, format!("{}:{}", img.repository, img.image_tag));
}

#[test]
fn http_port() {
    let reg = test_registry();
    let wildfly_image = reg.get(340).unwrap();
    assert_eq!(wildfly_image.http_port(), 8340);
}

#[test]
fn management_port() {
    let reg = test_registry();
    let wildfly_image = reg.get(340).unwrap();
    assert_eq!(wildfly_image.management_port(), 9340);
}

#[test]
fn platforms() {
    let reg = test_registry();
    let old = reg.get(100).unwrap();
    assert!(old.platforms.is_empty());
    let new = reg.get(390).unwrap();
    assert_eq!(new.platforms.len(), 4);
}

// ------------------------------------------------------ ordering

#[test]
fn ordering() {
    let reg = test_registry();
    let a = reg.get(100).unwrap();
    let b = reg.get(390).unwrap();
    assert!(a < b);
}

#[test]
fn partial_ord_consistent_with_ord() {
    let reg = test_registry();
    let a = reg.get(100).unwrap();
    let b = reg.get(200).unwrap();
    assert_eq!(a.partial_cmp(b), Some(Ordering::Less));
    assert_eq!(b.partial_cmp(a), Some(Ordering::Greater));
    assert_eq!(a.partial_cmp(a), Some(Ordering::Equal));
}

// ------------------------------------------------------ stability support

#[test]
fn supports_stability_below_threshold() {
    let reg = test_registry();
    let img = reg.get(300).unwrap(); // WildFly 30.0
    assert!(!img.supports_stability());
}

#[test]
fn supports_stability_at_threshold() {
    let reg = test_registry();
    let img = reg.get(310).unwrap(); // WildFly 31.0
    assert!(img.supports_stability());
}

#[test]
fn supports_stability_above_threshold() {
    let reg = test_registry();
    let img = reg.get(390).unwrap(); // WildFly 39.0
    assert!(img.supports_stability());
}

#[test]
fn supports_stability_dev() {
    let dev = wildfly_dev();
    assert!(dev.supports_stability());
}
