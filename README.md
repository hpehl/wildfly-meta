# WildFly Meta

A Rust library for managing WildFly metadata: container images, feature packs, and version expression parsing.

Data is loaded from TOML configuration files stored in `~/.config/wildfly-meta/` and downloaded on demand from GitHub. The library is consumed by Rust-based CLI tools such as [wado](https://github.com/hpehl/wado) and [mgt](https://github.com/hpehl/wildfly-model-graph).

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
wildfly_meta = "0.1"
```

```rust
use anyhow::Result;
use wildfly_meta::{
    update_all, WildFlyImageRegistry, FeaturePackRegistry,
    parse_list, ParseOptions, MetaItem,
};

fn main() -> Result<()> {
    // Download / update configuration files
    let result = update_all()?;
    println!("{}", result.summary());

    // Load registries
    let images = WildFlyImageRegistry::load_default()?;
    let packs = FeaturePackRegistry::load_default()?;

    // Parse a mixed expression
    let items = parse_list("34,35,ai", &images, &packs, &ParseOptions::all())?;
    for item in &items {
        println!("{}", item.full_name());
    }
    Ok(())
}
```

## Container Images

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
use wildfly_meta::{parse_image, parse_feature_pack, parse_item};

// Parse a single image: "dev", "34", or "26.1"
let img = parse_image("34", &images)?;

// Parse a single feature pack: "ai" (latest) or "ai:0.9.0" (specific version)
let fp = parse_feature_pack("ai", &packs)?;

// Parse either type (feature packs take priority on name collision)
let item = parse_item("ai", &images, &packs)?;
```

### Lists and Expressions

`parse_list` parses comma-separated expressions with optional support for ranges (`..`) and multipliers (`Nx`), controlled by `ParseOptions`:

```rust
use wildfly_meta::{parse_list, ParseOptions};

let options = ParseOptions::all();  // enable ranges and multipliers

// Plain versions and feature packs
let items = parse_list("34,35,ai", &images, &packs, &options)?;

// Ranges: all versions from 23 to 26
let items = parse_list("23..26", &images, &packs, &options)?;

// Open ranges: from 30 to newest, or oldest to 26
let items = parse_list("30..", &images, &packs, &options)?;
let items = parse_list("..26", &images, &packs, &options)?;

// Multipliers: three copies of version 34
let items = parse_list("3x34", &images, &packs, &options)?;

// Complex mixed expression
let items = parse_list("3x10,23..26,5x28,34,dev,ai", &images, &packs, &options)?;
```

`ParseOptions` controls which syntax elements are enabled:

| Option | Enables |
|--------|---------|
| `ParseOptions::all()` | Ranges and multipliers |
| `ParseOptions::none()` | Plain versions and feature packs only |

### `MetaItem`

`MetaItem` is the unified enum returned by `parse_item` and `parse_list`:

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
use wildfly_meta::{update_all, update_images, update_feature_packs, UpdateStatus};

// Update both files at once
let result = update_all()?;
println!("{}", result.summary());

// Update individually
let status = update_images()?;
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
| `images_path()` | `~/.config/wildfly-meta/wildfly-images.toml` |
| `feature_packs_path()` | `~/.config/wildfly-meta/feature-packs.toml` |

## Shell Completion

The library provides helpers for implementing shell tab-completion in CLI tools.

```rust
use wildfly_meta::{all_identifiers, suggest, CompletionOptions};

let options = CompletionOptions {
    feature_packs: true,
    ranges: true,
};

// Get all available identifiers for completion
let ids = all_identifiers(&images, &packs, &options);

// Get context-aware suggestions for partial input
let suggestions = suggest("34,", &images, &packs, &options);   // fresh after comma
let suggestions = suggest("20..", &images, &packs, &options);   // range completion
```

## Data Files

Two TOML files in the repository root serve as the canonical data source:

### `wildfly-images.toml`

```toml
config_version = 5

[[images]]
major = 35
minor = 0
version = "35.0.1"
core_version = "27.0.1"
suffix = "Final-jdk21"
repository = "quay.io/wildfly/wildfly"
platforms = ["linux/amd64", "linux/arm64", "linux/s390x", "linux/ppc64le"]
```

To add a new WildFly version, append an `[[images]]` entry and increment `config_version`. No code changes or library release needed.

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

## Supported Versions

| Short Version | WildFly Version | Image / Tag                                  | Platforms                                            |
|---------------|-----------------|----------------------------------------------|------------------------------------------------------|
| 10            | 10.0.0.Final    | docker.io/jboss/wildfly:10.0.0.Final         |                                                      |
| 10.1          | 10.1.0.Final    | docker.io/jboss/wildfly:10.1.0.Final         |                                                      |
| 11            | 11.0.0.Final    | docker.io/jboss/wildfly:11.0.0.Final         |                                                      |
| 12            | 12.0.0.Final    | docker.io/jboss/wildfly:12.0.0.Final         |                                                      |
| 13            | 13.0.0.Final    | docker.io/jboss/wildfly:13.0.0.Final         |                                                      |
| 14            | 14.0.1.Final    | docker.io/jboss/wildfly:14.0.1.Final         |                                                      |
| 15            | 15.0.1.Final    | docker.io/jboss/wildfly:15.0.1.Final         |                                                      |
| 16            | 16.0.0.Final    | docker.io/jboss/wildfly:16.0.0.Final         |                                                      |
| 17            | 17.0.1.Final    | docker.io/jboss/wildfly:17.0.1.Final         |                                                      |
| 18            | 18.0.1.Final    | docker.io/jboss/wildfly:18.0.1.Final         |                                                      |
| 19            | 19.0.0.Final    | docker.io/jboss/wildfly:19.0.0.Final         |                                                      |
| 19.1          | 19.1.0.Final    | docker.io/jboss/wildfly:19.1.0.Final         |                                                      |
| 20            | 20.0.1.Final    | docker.io/jboss/wildfly:20.0.1.Final         |                                                      |
| 21            | 21.0.2.Final    | docker.io/jboss/wildfly:21.0.2.Final         |                                                      |
| 22            | 22.0.1.Final    | docker.io/jboss/wildfly:22.0.1.Final         |                                                      |
| 23            | 23.0.2.Final    | quay.io/wildfly/wildfly:23.0.2.Final         |                                                      |
| 24            | 24.0.1.Final    | quay.io/wildfly/wildfly:24.0.1.Final         |                                                      |
| 25            | 25.0.1.Final    | quay.io/wildfly/wildfly:25.0.1.Final         |                                                      |
| 26            | 26.0.1.Final    | quay.io/wildfly/wildfly:26.0.1.Final         |                                                      |
| 26.1          | 26.1.3.Final    | quay.io/wildfly/wildfly:26.1.3.Final-jdk17   | linux/amd64, linux/arm64                             |
| 27            | 27.0.1.Final    | quay.io/wildfly/wildfly:27.0.1.Final-jdk19   | linux/amd64, linux/arm64                             |
| 28            | 28.0.1.Final    | quay.io/wildfly/wildfly:28.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 29            | 29.0.1.Final    | quay.io/wildfly/wildfly:29.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 30            | 30.0.1.Final    | quay.io/wildfly/wildfly:30.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 31            | 31.0.1.Final    | quay.io/wildfly/wildfly:31.0.1.Final-jdk20   | linux/amd64, linux/arm64                             |
| 32            | 32.0.1.Final    | quay.io/wildfly/wildfly:32.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x                |
| 33            | 33.0.2.Final    | quay.io/wildfly/wildfly:33.0.2.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 34            | 34.0.1.Final    | quay.io/wildfly/wildfly:34.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 35            | 35.0.1.Final    | quay.io/wildfly/wildfly:35.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 36            | 36.0.1.Final    | quay.io/wildfly/wildfly:36.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 37            | 37.0.1.Final    | quay.io/wildfly/wildfly:37.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 38            | 38.0.1.Final    | quay.io/wildfly/wildfly:38.0.1.Final-jdk21   | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
| 39            | 39.0.1.Final    | quay.io/wildfly/wildfly:39.0.1.Final-2-jdk21 | linux/amd64, linux/arm64, linux/s390x, linux/ppc64le |
