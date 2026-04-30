# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes

- Changed `FeaturePack.version` from `String` to `semver::Version` for type-safe semantic versioning
- Changed `FeaturePackRegistry` BTreeMap key from `(String, String)` to `(String, semver::Version)`, giving proper version ordering
- `FeaturePackRegistry::keys()` now returns `impl Iterator<Item = &(String, semver::Version)>`
- `FeaturePackRegistry::known_versions()` now returns `Vec<String>` instead of `Vec<&str>`

## [0.2.0] - 2026-04-30

### Breaking Changes

- Renamed TOML key `[[images]]` to `[[wildfly_images]]` in `wildfly-images.toml`
- Renamed constant `IMAGES_FILENAME` to `WILDFLY_IMAGES_FILENAME`
- Renamed function `images_path()` to `wildfly_images_path()`
- Renamed function `update_images()` to `update_wildfly_images()`
- Renamed function `update_images_with_base_url()` to `update_wildfly_images_with_base_url()`
- Renamed field `UpdateResult.images` to `UpdateResult.wildfly_images`
- `parse_meta_items` and `suggest_meta_items` now take separate options for WildFly images and feature packs

### Changed

- Consistent naming throughout the codebase: `wildfly_image`/`feature_pack` instead of abbreviated forms
- Feature pack tests are now data-independent

## [0.1.0] - 2026-04-29

- First release 🎉

[Unreleased]: https://github.com/hpehl/wildfly-meta/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/hpehl/wildfly-meta/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/hpehl/wildfly-meta/releases/tag/v0.1.0
