//! Evaluation sets, seals, matrix runner, execution descriptor, durable ledger, metrics, bootstrap
//! and gates. The statistics contract (`bootstrap`, `wilson`, `gate_rules`) is frozen first, in
//! task 1.15, so every later result is reproducible from its recorded seed.

pub mod admissions;
pub mod arms;
pub mod bootstrap;
pub mod canary;
pub mod descriptor;
pub mod estimate;
pub mod eval_ledger;
pub mod fixture;
pub mod gate;
pub mod gate_rules;
pub mod matrix;
pub mod metrics;
pub mod runner;
pub mod sets;
pub mod summary;
pub mod wilson;

/// Errors this crate produces. Every public fallible function returns this type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A statistics fixture or result table could not be read or is inconsistent.
    #[error("stats: {0}")]
    Stats(String),
    /// An evaluation set, its manifest, labels, images or seal is missing or inconsistent.
    #[error("sets: {0}")]
    Sets(String),
    /// A matrix file is missing, malformed or inconsistent.
    #[error("matrix: {0}")]
    Matrix(String),
    /// The execution descriptor or the writer lock of an evaluation directory.
    #[error("descriptor: {0}")]
    Descriptor(String),
    /// The durable evaluation ledger could not be read or written.
    #[error("ledger: {0}")]
    Ledger(String),
    /// The admission record is missing, malformed or inconsistent.
    #[error("admissions: {0}")]
    Admissions(String),
    /// The matrix runner: an arm that cannot be formed, a config or job that does not resolve, or
    /// an evaluation directory that cannot be written.
    #[error("runner: {0}")]
    Runner(String),
    /// The gate lifecycle: a refused scoring, a receipt or publication that cannot be written.
    #[error("gate: {0}")]
    Gate(String),
    /// The canary: no baseline, a configuration the baseline does not cover, or state that
    /// cannot be read or written.
    #[error("canary: {0}")]
    Canary(String),
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Writes a file through a temporary neighbour, synced and renamed into place, so a crash
/// mid-write leaves the old file or none, never a truncated one.
pub fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension(format!(
        "{}.tmp-{}",
        path.extension().and_then(|e| e.to_str()).unwrap_or("file"),
        std::process::id()
    ));
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}
