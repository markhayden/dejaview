//! Evaluation sets, seals, matrix runner, execution descriptor, durable ledger, metrics, bootstrap
//! and gates. The statistics contract (`bootstrap`, `wilson`, `gate_rules`) is frozen first, in
//! task 1.15, so every later result is reproducible from its recorded seed.

pub mod bootstrap;
pub mod fixture;
pub mod gate_rules;
pub mod wilson;

/// Errors this crate produces. Every public fallible function returns this type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A statistics fixture or result table could not be read or is inconsistent.
    #[error("stats: {0}")]
    Stats(String),
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;
