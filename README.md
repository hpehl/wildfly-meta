# WildFly Meta

A Rust library for managing WildFly metadata: container images, feature packs, and version expression parsing.

Data is loaded from TOML configuration files stored in `~/.config/wildfly-meta/` and downloaded on demand from GitHub. The library is consumed by Rust-based CLI tools such as [wado](https://github.com/hpehl/wado) and [mgt](https://github.com/model-graph-tools/tooling).

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
wildfly_meta = "0.2"
```

```rust
use anyhow::Result;
use wildfly_meta::{
    update_all, WildFlyImageRegistry, FeaturePackRegistry,
    parse_meta_items, ParseOptions, MetaItem,
};

fn main() -> Result<()> {
    // Download / update configuration files
    let result = update_all()?;
    println!("{}", result.summary());

    // Load registries
    let wildfly_images = WildFlyImageRegistry::load_default()?;
    let feature_packs = FeaturePackRegistry::load_default()?;

    // Parse a mixed expression
    let items = parse_meta_items("34,35,ai", &wildfly_images, &feature_packs, &ParseOptions::all(), &ParseOptions::all())?;
    for item in &items {
        println!("{}", item.full_name());
    }
    Ok(())
}
```

## WildFly Images

Container images represent WildFly versions deployed at [Docker Hub](https://hub.docker.com/r/jboss/wildfly) and [Quay.io](https://quay.io/repository/wildfly/wildfly).

### `WildFlyImage`

```rust
pub struct WildFlyImage {
    pub identifier: u16,          // numeric ID (major * 10 + minor)
    pub version: Version,         // full semver (e.g. 26.1.3)
    pub short_version: String,    // display version (e.g. "26.1")
    pub core_version: Version,    // WildFly Core version
    pub suffix: String,           // tag suffix (e.g. "Final-jdk21")
    pub repository: String,       // container registry URL
    pub platforms: Vec<String>,   // supported platforms
}
```

Key methods:

| Method              | Description                                                              |
|---------------------|--------------------------------------------------------------------------|
| `image_ref()`       | Full image reference (e.g. `quay.io/wildfly/wildfly:35.0.1.Final-jdk21`) |
| `is_dev()`          | `true` for the development version                                       |
| `short_name()`      | `"dev"` or the short version string (`"10"`, `"26.1"`, `"39"`)           |
| `full_name()`       | `"WildFly dev"` or `"WildFly 34.0"` for releases                        |
| `http_port()`       | Computed HTTP port (8000 + offset)                                       |
| `management_port()` | Computed management port (9000 + offset)                                 |

### `WildFlyImageRegistry`

Loads and queries images from `wildfly-images.toml`.

```rust
let images = WildFlyImageRegistry::load_default()?;        // from ~/.config/wildfly-meta/
let images = WildFlyImageRegistry::load(Path::new("…"))?;  // from custom path
let images = WildFlyImageRegistry::from_toml(content)?;    // from TOML string

let img = images.get(350);                           // lookup by identifier
let first = images.first();                          // oldest version
let last = images.last();                            // newest version
let range = images.range(260, 290);                  // versions 26.0 – 29.0
let all = images.all();                              // all versions
```

### Helper Functions

- `wildfly_dev()` — creates the special development `WildFlyImage` instance
- `identifier(major, minor)` — computes the numeric identifier (`major * 10 + minor`)

## Feature Packs

Feature packs represent WildFly Galleon feature packs with Maven coordinates.

### `FeaturePack`

```rust
pub struct FeaturePack {
    pub shortcut: String,        // short name (e.g. "ai", "graphql")
    pub name: String,            // display name (e.g. "AI", "GraphQL")
    pub group_id: String,        // Maven group ID
    pub artifact_id: String,     // Maven artifact ID
    pub version: String,         // version string
    pub maven_version: String,   // Maven artifact version
    pub shortcut_index: u16,     // computed index for port offset
    pub version_index: u16,      // version sequence number
}
```

Key methods:

| Method           | Description                                                         |
|------------------|---------------------------------------------------------------------|
| `port_offset()`    | Computed port offset (10000 + shortcut_index * 100 + version_index) |
| `container_name()` | Container-safe name (e.g. `ai-0-9-0`)                               |
| `download_url()`   | Maven Central URL for the doc archive                               |
| `short_name()`     | `"shortcut version"` format (e.g. `"ai 0.9.0"`)                    |
| `full_name()`      | Branded name (e.g. `"AI Feature Pack 0.9.0"`)                      |

### `FeaturePackRegistry`

Loads and queries feature packs from `feature-packs.toml`.

```rust
let packs = FeaturePackRegistry::load_default()?;

let fp = packs.get("ai", "0.9.0");          // lookup by shortcut + version
let latest = packs.latest("ai");            // latest version of a shortcut
let shortcuts = packs.known_shortcuts();     // all unique shortcuts
let versions = packs.known_versions("ai");   // all versions for a shortcut
let all = packs.all();                       // all feature packs
let ids = packs.all_identifiers();           // all "shortcut" and "shortcut:version" strings
```

## Parsing

The library provides parsing functions for version expressions that can reference both container images and feature packs.

### Single Items

```rust
use wildfly_meta::{parse_wildfly_image, parse_feature_pack, parse_meta_item};

// Parse a single WildFly image: "dev", "34", or "26.1"
let wildfly_image = parse_wildfly_image("34", &wildfly_images)?;

// Parse a single feature pack: "ai" (latest) or "ai:0.9.0" (specific version)
let feature_pack = parse_feature_pack("ai", &feature_packs)?;

// Parse either type (feature packs take priority on name collision)
let meta_item = parse_meta_item("ai", &wildfly_images, &feature_packs)?;
```

### Lists and Expressions

`parse_meta_items` parses comma-separated expressions with optional support for ranges (`..`) and multipliers (`Nx`), controlled by separate `ParseOptions` for WildFly images and feature packs:

```rust
use wildfly_meta::{parse_meta_items, ParseOptions};

let image_options = ParseOptions::all();   // enable ranges and multipliers for images
let fp_options = ParseOptions::all();      // enable multipliers for feature packs

// Plain versions and feature packs
let items = parse_meta_items("34,35,ai", &wildfly_images, &feature_packs, &image_options, &fp_options)?;

// Ranges: all versions from 23 to 26
let items = parse_meta_items("23..26", &wildfly_images, &feature_packs, &image_options, &fp_options)?;

// Open ranges: from 30 to newest, or oldest to 26
let items = parse_meta_items("30..", &wildfly_images, &feature_packs, &image_options, &fp_options)?;
let items = parse_meta_items("..26", &wildfly_images, &feature_packs, &image_options, &fp_options)?;

// Multipliers: three copies of version 34
let items = parse_meta_items("3x34", &wildfly_images, &feature_packs, &image_options, &fp_options)?;

// Complex mixed expression
let items = parse_meta_items("3x10,23..26,5x28,34,dev,ai", &wildfly_images, &feature_packs, &image_options, &fp_options)?;
```

`ParseOptions` controls which syntax elements are enabled:

| Option | Enables |
|--------|---------|
| `ParseOptions::all()` | Ranges and multipliers |
| `ParseOptions::none()` | Plain versions and feature packs only |

### `MetaItem`

`MetaItem` is the unified enum returned by `parse_meta_item` and `parse_meta_items`:

```rust
pub enum MetaItem {
    Image(WildFlyImage),
    FeaturePack(FeaturePack),
}
```

| Method | Description |
|--------|-------------|
| `short_name()` | Short display string for the item |
| `full_name()` | Branded label (e.g. `"WildFly 34.0"` or `"AI Feature Pack 0.9.0"`) |
| `port_offset()` | Port offset (identifier for images, computed for feature packs) |
| `container_name()` | Container-safe name |
| `kind()` | `"wildfly"` or `"feature-pack"` |
| `expression()` | Re-parseable expression (version or `shortcut:version`) |

## Configuration Update

Configuration files are downloaded from the [wildfly-meta](https://github.com/hpehl/wildfly-meta) repository on GitHub and stored in `~/.config/wildfly-meta/`. A `config_version` field in each TOML file controls whether a re-download is needed.

```rust
use wildfly_meta::{update_all, update_wildfly_images, update_feature_packs, UpdateStatus};

// Update both files at once
let result = update_all()?;
println!("{}", result.summary());

// Update individually
let status = update_wildfly_images()?;
match status {
    UpdateStatus::Downloaded { version, count } => { /* first download */ }
    UpdateStatus::Updated { from_version, to_version, diff } => {
        println!("Added: {:?}", diff.added);
        println!("Removed: {:?}", diff.removed);
    }
    UpdateStatus::AlreadyUpToDate(version) => { /* no changes */ }
}
```

Path helpers:

| Function | Returns |
|----------|---------|
| `config_dir()` | `~/.config/wildfly-meta` |
| `wildfly_images_path()` | `~/.config/wildfly-meta/wildfly-images.toml` |
| `feature_packs_path()` | `~/.config/wildfly-meta/feature-packs.toml` |

## Shell Completion

The library provides helpers for implementing shell tab-completion in CLI tools.

```rust
use wildfly_meta::{
    all_wildfly_images, all_feature_packs, all_meta_items,
    suggest_wildfly_images, suggest_feature_packs, suggest_meta_items,
    CompletionOptions,
};

let options = CompletionOptions {
    ranges: true,
    multipliers: true,
};

// Get all available identifiers for completion
let ids = all_meta_items(&wildfly_images, &feature_packs);

// Get context-aware suggestions for partial input
let suggestions = suggest_meta_items("34,", &wildfly_images, &feature_packs, &options, &options);
let suggestions = suggest_meta_items("20..", &wildfly_images, &feature_packs, &options, &options);
```

## Data Files

Two TOML files in the repository root serve as the canonical data source:

### `wildfly-images.toml`

```toml
config_version = 5

[[wildfly_images]]
major = 35
minor = 0
version = "35.0.1"
core_version = "27.0.1"
suffix = "Final-jdk21"
repository = "quay.io/wildfly/wildfly"
platforms = ["linux/amd64", "linux/arm64", "linux/s390x", "linux/ppc64le"]
```

To add a new WildFly version, append a `[[wildfly_images]]` entry and increment `config_version`. No code changes or library release needed.

### `feature-packs.toml`

```toml
config_version = 3

[[feature_packs]]
shortcut = "ai"
name = "AI"
group_id = "org.wildfly.generative-ai"
artifact_id = "wildfly-ai-feature-pack"
version = "0.9.0"
maven_version = "0.9.0"
```

To add a new feature pack, append a `[[feature_packs]]` entry and increment `config_version`. `shortcut_index` and `version_index` are computed at load time from TOML order.
