# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-29

### Changed

- Replace static version map with TOML-based registries (`wildfly-images.toml`,
  `feature-packs.toml`) loaded at runtime
- Add on-demand download of TOML configuration files from GitHub to
  `~/.config/wildfly-meta/`
- Add unified expression parser supporting a mini-DSL:
  `"3x10,23..26,5x28,34,dev,ai"`
- Split library into focused modules: `image`, `feature_pack`, `parse`, `update`

[Unreleased]: https://github.com/hpehl/wildfly-meta/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/hpehl/wildfly-meta/releases/tag/v0.1.0
