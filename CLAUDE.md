# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust library crate (`wildfly_meta`) for managing WildFly metadata: container images, feature packs, and version expression parsing. Data is loaded from TOML configuration files rather than being hardcoded. Consumed by CLI tools [wado](https://github.com/hpehl/wado) and [mgt](https://github.com/hpehl/wildfly-model-graph). Published to [crates.io](https://crates.io/crates/wildfly_meta).

Repository: [github.com/hpehl/wildfly-meta](https://github.com/hpehl/wildfly-meta)

## Build & Test Commands

```bash
cargo build          # build the library
cargo test           # run all tests
cargo test <name>    # run a single test by name
cargo clippy         # lint
cargo fmt --check    # check formatting
```

## Architecture

Multi-file library crate with no binary targets:

- **`src/wildfly_image.rs`** — `WildFlyImage` struct, `WildFlyImageRegistry` for loading/querying container images from TOML, `wildfly_dev()` helper, and `identifier()` for computing version IDs
- **`src/feature_pack.rs`** — `FeaturePack` struct and `FeaturePackRegistry` for loading/querying feature pack metadata from TOML
- **`src/meta_item.rs`** — `MetaItem` enum wrapping both `WildFlyImage` and `FeaturePack` with unified accessors
- **`src/parse.rs`** — Unified expression parser supporting a mini-DSL: `"3x10,23..26,5x28,34,dev,ai"`. Separate `ParseOptions` for WildFly images and feature packs
- **`src/complete.rs`** — Shell completion helpers: `suggest_wildfly_images`, `suggest_feature_packs`, `suggest_meta_items`, and `all_*` identifier functions. `CompletionOptions` controls ranges and multipliers
- **`src/update.rs`** — On-demand download of TOML files from GitHub (`main` branch) to `~/.config/wildfly-meta/`. Compares `config_version` to decide whether to re-download. Exports `UpdateStatus` enum
- **`src/lib.rs`** — Public re-exports of all types, functions, and constants

## Data Files

Two TOML files in the repository root serve as the canonical data source:

- **`wildfly-images.toml`** — WildFly container image metadata (version, core_version, suffix, repository, platforms)
- **`feature-packs.toml`** — Feature pack metadata (shortcut, name, Maven coordinates)

Both contain a `config_version` field incremented on each update. The library downloads these to `~/.config/wildfly-meta/` on demand.

## Adding a New WildFly Version

Add a new `[[images]]` entry to `wildfly-images.toml` and increment `config_version`. No code changes or library release needed.

## Adding a New Feature Pack

Add a new `[[feature_packs]]` entry to `feature-packs.toml` and increment `config_version`. `shortcut_index` and `version_index` are computed at load time from TOML order.

## Release Process

1. Add changes under `## [Unreleased]` in `CHANGELOG.md`
2. Run `./release.sh <semver>` which uses `cargo-release` (configured in `release.toml`) to:
   - Bump the version in `Cargo.toml`
   - Stamp `[Unreleased]` in `CHANGELOG.md` with the version and date
   - Run `cargo fmt`
   - Commit, tag (`v<semver>`), and push to trigger the GitHub release workflow
3. The GitHub workflow (`.github/workflows/release.yml`) creates a release from `CHANGELOG.md` and publishes to crates.io

CI runs on every push via `.github/workflows/verify.yml`.

To undo a failed release, run `./unrelease.sh <version>` to delete the tag.
