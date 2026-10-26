//! `dejaview report`: rebuild `report.json` from a decision directory's header and journals, or
//! prune decision directories by age.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use clap::Subcommand;
use dejaview_core::config::Config;
use dejaview_core::report::{self, Report};

use crate::EXIT_USAGE;

#[derive(clap::Args)]
pub struct ReportArgs {
    #[command(subcommand)]
    pub command: ReportCommand,
}

#[derive(Subcommand)]
pub enum ReportCommand {
    /// Rebuild `report.json` from the decision header and the run journals, complete or partial.
    Rebuild(RebuildArgs),
    /// Remove decision directories at least this old (`30d`, `12h`, `15m`, `0s`).
    Prune(PruneArgs),
}

#[derive(clap::Args)]
pub struct RebuildArgs {
    /// A decision directory (`.../decision-<ms>-<pid>`).
    pub dir: PathBuf,
}

#[derive(clap::Args)]
pub struct PruneArgs {
    /// The output directory holding decision directories; defaults to the config's `[output] dir`.
    #[arg(long, required_unless_present = "config")]
    pub out: Option<PathBuf>,
    /// The config whose `[output] dir` to prune.
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Age threshold: `30d`, `12h`, `15m` or `0s`.
    #[arg(long, value_parser = parse_duration)]
    pub older_than: Duration,
}

pub fn run(args: &ReportArgs) -> i32 {
    match &args.command {
        ReportCommand::Rebuild(a) => rebuild(a),
        ReportCommand::Prune(a) => prune(a),
    }
}

fn rebuild(args: &RebuildArgs) -> i32 {
    let report = match Report::rebuild(&args.dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    if let Err(e) = report.write(&args.dir) {
        eprintln!("dejaview: {e}");
        return EXIT_USAGE;
    }
    for run in &report.runs {
        let detail = run
            .error
            .as_ref()
            .map(|e| format!(" ({}: {})", e.class, e.message))
            .unwrap_or_default();
        println!("run-{}: {}{detail}", run.run, run.terminal);
    }
    let verdict = report
        .verdict
        .and_then(|v| serde_json::to_string(&v).ok())
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_else(|| "none".into());
    println!("verdict: {verdict}");
    println!("report: {}", args.dir.join(report::REPORT_FILE).display());
    0
}

fn prune(args: &PruneArgs) -> i32 {
    let out = match (&args.out, &args.config) {
        (Some(out), _) => out.clone(),
        (None, Some(config)) => match Config::load(config) {
            Ok(c) => c.output.dir.clone(),
            Err(e) => {
                eprintln!("dejaview: {e}");
                return EXIT_USAGE;
            }
        },
        (None, None) => {
            eprintln!("dejaview: pass --out or --config");
            return EXIT_USAGE;
        }
    };
    match report::prune(&out, args.older_than, SystemTime::now()) {
        Ok(removed) => {
            for p in &removed {
                println!("removed {}", p.display());
            }
            println!(
                "pruned {} decision(s) under {}",
                removed.len(),
                out.display()
            );
            0
        }
        Err(e) => {
            eprintln!("dejaview: {e}");
            EXIT_USAGE
        }
    }
}

/// `30d`, `12h`, `15m`, `0s`.
pub fn parse_duration(text: &str) -> Result<Duration, String> {
    let text = text.trim();
    let (number, unit) = text.split_at(text.len().saturating_sub(1));
    let n: u64 = number
        .parse()
        .map_err(|_| format!("{text:?}: expected a number followed by d, h, m or s"))?;
    let seconds = match unit {
        "d" => n * 86_400,
        "h" => n * 3_600,
        "m" => n * 60,
        "s" => n,
        _ => return Err(format!("{text:?}: the unit must be d, h, m or s")),
    };
    Ok(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_parse_with_their_units() {
        assert_eq!(parse_duration("30d"), Ok(Duration::from_secs(30 * 86_400)));
        assert_eq!(parse_duration("12h"), Ok(Duration::from_secs(12 * 3_600)));
        assert_eq!(parse_duration("15m"), Ok(Duration::from_secs(900)));
        assert_eq!(parse_duration("0s"), Ok(Duration::ZERO));
        assert!(parse_duration("3w").is_err());
        assert!(parse_duration("d").is_err());
    }
}
