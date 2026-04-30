//! Shared options controlling which DSL features are enabled during parsing and completion.

/// Controls which DSL features are enabled during parsing and shell completion.
pub struct DslOptions {
    /// Whether range expressions like `20..25` are allowed.
    pub ranges: bool,
    /// Whether multiplier prefixes like `3x34` are allowed.
    pub multipliers: bool,
}

impl DslOptions {
    /// Enables all DSL features (ranges and multipliers).
    pub fn all() -> Self {
        Self {
            ranges: true,
            multipliers: true,
        }
    }

    /// Disables all DSL features — only plain version and feature pack references are accepted.
    pub fn none() -> Self {
        Self {
            ranges: false,
            multipliers: false,
        }
    }
}

impl Default for DslOptions {
    fn default() -> Self {
        Self::all()
    }
}
