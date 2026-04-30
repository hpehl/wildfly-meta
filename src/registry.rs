//! Shared helpers for loading TOML-based registries.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::de::DeserializeOwned;

use crate::update::UpdateStatus;

pub(crate) fn load_toml<T>(
    path: &Path,
    resolution_hint: &str,
    parse: impl Fn(&str) -> Result<T>,
) -> Result<T> {
    let content = fs::read_to_string(path).map_err(|e| {
        if resolution_hint.is_empty() {
            anyhow::anyhow!("{e}")
        } else {
            anyhow::anyhow!("{e}. {resolution_hint}")
        }
    })?;
    parse(&content).map_err(|e| {
        if resolution_hint.is_empty() {
            anyhow::anyhow!("Failed to parse {}: {e}", path.display())
        } else {
            anyhow::anyhow!("Failed to parse {}: {e}. {resolution_hint}", path.display())
        }
    })
}

pub(crate) fn load_or_update<T>(
    path: PathBuf,
    resolution_hint: &str,
    update_fn: impl Fn() -> Result<UpdateStatus>,
    load_default: impl Fn(&str) -> Result<T>,
) -> Result<T> {
    if !path.exists() {
        update_fn()?;
    }
    load_default(resolution_hint).or_else(|_| {
        update_fn()?;
        load_default(resolution_hint)
    })
}

pub(crate) fn config_version<C: DeserializeOwned>(
    path: &Path,
    extract: impl Fn(&C) -> u32,
) -> Result<u32> {
    let content = fs::read_to_string(path)?;
    let config: C = toml::from_str(&content)?;
    Ok(extract(&config))
}
