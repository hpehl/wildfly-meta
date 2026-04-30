# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.1] - 2026-04-30

### Fixed

- Fix test assertions referencing removed Keycloak feature pack

## [0.6.0] - 2026-04-30

### Added

- Derive `Serialize` on `WildFlyImage`, `FeaturePack`, and `MetaItem` for JSON serialization support

### Changed

- Extract tests into separate files for `wildfly_image`, `feature_pack`, and `update` modules

### Removed

- Remove Keycloak feature pack and GraphQL 2.5.0 version (no published doc-zip artifact)

## [0.5.0] - 2026-04-30

### Breaking Changes

- Renamed `ParseOptions` and `CompletionOptions` to unified `DslOptions` struct

### Changed

- Extracted shared registry loading and `config_version` logic into internal `registry` module
- Extracted `compute_registry_diff` generic helper to eliminate duplicated diff logic in `update.rs`
- Simplified `FeaturePackRegistry::from_toml` index tracking using `entry` API
- Moved tests from `parse.rs` and `complete.rs` into dedicated `parse/tests.rs` and `complete/tests.rs` submodules
- Added `Hash` derive to `FeaturePack` for consistency with `WildFlyImage`

## [0.4.1] - 2026-04-30

### Added

- `WildFlyImageRegistry::load_or_update()` and `FeaturePackRegistry::load_or_update()` convenience methods that automatically download the configuration if it is missing or corrupt, retrying once on load failure
- `UpdateStatus::summary()` is now public, allowing consumers to generate human-readable status messages

## [0.4.0] - 2026-04-30

### Breaking Changes

- `WildFlyImageRegistry::load_default()` and `WildFlyImageRegistry::load()` now require a `resolution_hint: &str` parameter
- `FeaturePackRegistry::load_default()` and `FeaturePackRegistry::load()` now require a `resolution_hint: &str` parameter

### Added

- Fail-safe resolution for missing or corrupt TOML configuration files: `load()` and `load_default()` accept a `resolution_hint` that is appended to error messages, letting each consumer suggest its own recovery action (e.g. `"Run 'wado update' to fix this."`)
- Corrupt local configuration files are now automatically replaced during update instead of causing a permanent error

## [0.3.0] - 2026-04-30

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

[Unreleased]: https://github.com/hpehl/wildfly-meta/compare/v0.6.1...HEAD
[0.6.1]: https://github.com/hpehl/wildfly-meta/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/hpehl/wildfly-meta/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/hpehl/wildfly-meta/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/hpehl/wildfly-meta/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/hpehl/wildfly-meta/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/hpehl/wildfly-meta/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/hpehl/wildfly-meta/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/hpehl/wildfly-meta/releases/tag/v0.1.0
