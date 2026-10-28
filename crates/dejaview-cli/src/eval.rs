//! `dejaview eval`: the matrix runner (3.4b), the estimate (3.4c), the report (3.5a) and the
//! gate on a statistics fixture (1.15); the gate lifecycle arrives with 3.5b.

use std::path::{Path, PathBuf};

use clap::Subcommand;
use dejaview_core::config::Config;
use dejaview_core::provider::Provider;
use dejaview_core::run::Cancel;
use dejaview_core::tool::ToolRegistry;
use dejaview_eval::admissions::Admissions;
use dejaview_eval::arms::ToolSelection;
use dejaview_eval::fixture::StatsFixture;
use dejaview_eval::matrix::{Cell, Matrix};
use dejaview_eval::runner::{self, Factories, Plan};

use crate::{EXIT_USAGE, ProviderOptions, VERSION};

#[derive(clap::Args)]
pub struct EvalArgs {
    #[command(subcommand)]
    pub command: EvalCommand,
}

#[derive(Subcommand)]
pub enum EvalCommand {
    /// Run (or resume) a matrix: every item of its sets through every cell, never reading the
    /// cache, under the evaluation's durable ledger.
    Run(RunArgs),
    /// Print every decision's envelope, their sum and a cost range for a matrix, and warn when
    /// the sum exceeds the cap; nothing is sent and nothing is refused.
    Estimate(EstimateArgs),
    /// Regenerate `metrics.json` and `report.md` in an evaluation directory from its saved
    /// results and the sets' labels; nothing is sent.
    Report(ReportArgs),
    /// Apply the admission gate to a synthetic result table and print the interval and outcome.
    Gate(GateArgs),
}

#[derive(clap::Args)]
pub struct ReportArgs {
    /// The evaluation directory an `eval run` wrote.
    #[arg(long)]
    out: PathBuf,
    /// The evalsets root the summary's set ids resolve under.
    #[arg(long, default_value = "evalsets")]
    evalsets_root: PathBuf,
}

fn eval_report(args: &ReportArgs) -> i32 {
    match dejaview_eval::summary::regenerate(&args.out, &args.evalsets_root) {
        Ok(m) => {
            println!(
                "eval report: {} cells over labelled items, written to {} and {} in {}",
                m.cells.len(),
                dejaview_eval::summary::METRICS_FILE,
                dejaview_eval::summary::REPORT_FILE,
                args.out.display()
            );
            0
        }
        Err(e) => {
            eprintln!("dejaview: {e}");
            EXIT_USAGE
        }
    }
}

#[derive(clap::Args)]
pub struct RunArgs {
    /// The matrix file.
    #[arg(long)]
    matrix: PathBuf,
    /// The evaluation directory: descriptor, ledger, results and summary.
    #[arg(long)]
    out: PathBuf,
    /// The evalsets root the matrix's set ids resolve under.
    #[arg(long, default_value = "evalsets")]
    evalsets_root: PathBuf,
    /// The admission record arm D reads; defaults to `<evalsets root>/admissions.toml` when it
    /// exists.
    #[arg(long)]
    admissions: Option<PathBuf>,
    /// Scripted fake provider responses for every decision, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    provider_script: Option<PathBuf>,
    /// Scripted envelopes for the fake `fake_regions` tool, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    tool_script: Option<PathBuf>,
}

#[derive(clap::Args)]
pub struct EstimateArgs {
    /// The matrix file.
    #[arg(long)]
    matrix: PathBuf,
    /// The evalsets root the matrix's set ids resolve under.
    #[arg(long, default_value = "evalsets")]
    evalsets_root: PathBuf,
    /// The admission record arm D reads; defaults to `<evalsets root>/admissions.toml` when it
    /// exists.
    #[arg(long)]
    admissions: Option<PathBuf>,
    /// Print JSON instead of the text form.
    #[arg(long)]
    json: bool,
    /// Scripted fake provider, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    provider_script: Option<PathBuf>,
    /// Scripted envelopes for the fake `fake_regions` tool, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    tool_script: Option<PathBuf>,
}

/// The plan a matrix file describes, with its config and admission record loaded; `Err` holds
/// the usage message.
fn plan_from(
    matrix_path: &Path,
    out: &Path,
    evalsets_root: &Path,
    admissions: Option<&Path>,
) -> Result<Plan, String> {
    let matrix_text = std::fs::read_to_string(matrix_path)
        .map_err(|e| format!("cannot read {}: {e}", matrix_path.display()))?;
    let matrix =
        Matrix::parse(&matrix_text).map_err(|e| format!("{}: {e}", matrix_path.display()))?;
    let config_text = std::fs::read_to_string(&matrix.matrix.config).map_err(|e| {
        format!(
            "cannot read the matrix's config {}: {e}",
            matrix.matrix.config.display()
        )
    })?;
    let admissions_path = admissions.map(Path::to_path_buf).or_else(|| {
        let p = evalsets_root.join("admissions.toml");
        p.exists().then_some(p)
    });
    let admissions = match admissions_path.as_deref().map(Admissions::load) {
        Some(Ok(a)) => Some(a),
        Some(Err(e)) => return Err(e.to_string()),
        None => None,
    };
    Ok(Plan {
        eval_dir: out.to_path_buf(),
        matrix,
        matrix_text,
        config_text,
        evalsets_root: evalsets_root.to_path_buf(),
        admissions,
        harness_version: VERSION.to_string(),
        allow_fake: cfg!(feature = "testkit"),
    })
}

fn eval_estimate(args: &EstimateArgs) -> i32 {
    let plan = match plan_from(
        &args.matrix,
        Path::new("."),
        &args.evalsets_root,
        args.admissions.as_deref(),
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let factories = CliFactories {
        #[cfg(feature = "testkit")]
        provider_script: args.provider_script.clone(),
        #[cfg(feature = "testkit")]
        tool_script: args.tool_script.clone(),
    };
    let e = match dejaview_eval::estimate::estimate(&plan, &factories) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("dejaview: {err}");
            return EXIT_USAGE;
        }
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&e).unwrap_or_default());
    } else {
        println!(
            "eval estimate {}: {} decisions",
            e.matrix,
            e.decisions.len()
        );
        for d in &e.decisions {
            println!("  {:<70} envelope ${:.4}", d.id, d.envelope_usd);
        }
        println!(
            "envelope sum ${:.4}; cost range ${:.4} (every decision submits at once) to ${:.4} (every envelope spent); cap ${:.2}",
            e.envelope_sum_usd, e.floor_usd, e.envelope_sum_usd, e.cap_usd
        );
        if e.over_cap {
            println!(
                "warning: the envelope sum exceeds the cap by ${:.4}; completion is not guaranteed (admission is per decision, against what is left)",
                e.envelope_sum_usd - e.cap_usd
            );
        }
    }
    0
}

/// The binary's providers and tools, per cell.
struct CliFactories {
    #[cfg(feature = "testkit")]
    provider_script: Option<PathBuf>,
    #[cfg(feature = "testkit")]
    tool_script: Option<PathBuf>,
}

impl Factories for CliFactories {
    fn provider(&self, _cell: &Cell, config: &Config) -> dejaview_eval::Result<Box<dyn Provider>> {
        let mut config = config.clone();
        let opts = ProviderOptions {
            #[cfg(feature = "testkit")]
            provider_script: self.provider_script.as_deref(),
            ..ProviderOptions::default()
        };
        crate::provider_for(&mut config, &opts).map_err(dejaview_eval::Error::Runner)
    }

    fn tools(
        &self,
        _selection: &ToolSelection,
        config: &Config,
    ) -> dejaview_eval::Result<ToolRegistry> {
        #[cfg(feature = "testkit")]
        let script = self.tool_script.as_deref();
        #[cfg(not(feature = "testkit"))]
        let script = None;
        crate::tools_for_config(config, script).map_err(dejaview_eval::Error::Runner)
    }
}

fn eval_run(args: &RunArgs) -> i32 {
    let plan = match plan_from(
        &args.matrix,
        &args.out,
        &args.evalsets_root,
        args.admissions.as_deref(),
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let factories = CliFactories {
        #[cfg(feature = "testkit")]
        provider_script: args.provider_script.clone(),
        #[cfg(feature = "testkit")]
        tool_script: args.tool_script.clone(),
    };
    // Signals cancel between decisions and inside one; the runtime that listens stays alive for
    // the whole run while every decision runs on its own.
    let signals_runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("dejaview: cannot start the runtime: {e}");
            return EXIT_USAGE;
        }
    };
    let cancel = std::sync::Arc::new(Cancel::new());
    let signals = {
        let _guard = signals_runtime.enter();
        crate::install_signal_handlers()
    };
    signals_runtime.spawn(crate::cancel_on_signal(
        signals,
        std::sync::Arc::clone(&cancel),
    ));
    match runner::run(&plan, &factories, Some(&cancel)) {
        Ok(s) => {
            let errors = s
                .decisions
                .iter()
                .filter(|d| d.error_class.is_some())
                .count();
            println!(
                "eval run {}: {} decisions ({} completed now, {} already complete, {} partial redone, {} errors), ${:.4} of ${:.2} spent, results in {}",
                s.matrix,
                s.decisions.len(),
                s.completed,
                s.skipped,
                s.partial.len(),
                errors,
                s.spent_usd,
                s.cap_usd,
                args.out.display()
            );
            if s.cancelled {
                eprintln!("dejaview: cancelled; run the same command again to resume");
                dejaview_core::run::EXIT_ERROR
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("dejaview: {e}");
            EXIT_USAGE
        }
    }
}

#[derive(clap::Args)]
pub struct GateArgs {
    /// A statistics fixture (`fixtures/stats/case-1.toml`).
    #[arg(long)]
    fixture: PathBuf,
    /// Print JSON instead of the text form.
    #[arg(long)]
    json: bool,
}

pub fn run(args: &EvalArgs) -> i32 {
    match &args.command {
        EvalCommand::Run(a) => eval_run(a),
        EvalCommand::Estimate(a) => eval_estimate(a),
        EvalCommand::Report(a) => eval_report(a),
        EvalCommand::Gate(a) => gate(a),
    }
}

fn gate(args: &GateArgs) -> i32 {
    let evaluation = StatsFixture::load(&args.fixture).and_then(|f| f.evaluate());
    let e = match evaluation {
        Ok(e) => e,
        Err(err) => {
            eprintln!("dejaview: {err}");
            return EXIT_USAGE;
        }
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&e).unwrap_or_default());
        return 0;
    }
    let b = &e.bootstrap;
    println!(
        "C: detected {}/{} missed {} false rejects {} unsure {} errors {} cost ${:.2}",
        e.c.detected,
        e.c.expected_fail,
        e.c.missed,
        e.c.false_rejects,
        e.c.unsure,
        e.c.errors,
        e.c.cost_usd
    );
    println!(
        "X: detected {}/{} missed {} false rejects {} unsure {} errors {} cost ${:.2}",
        e.x.detected,
        e.x.expected_fail,
        e.x.missed,
        e.x.false_rejects,
        e.x.unsure,
        e.x.errors,
        e.x.cost_usd
    );
    println!(
        "paired bootstrap: point {:.4}, interval [{:.4}, {:.4}] at percentiles {} and {}, {} resamples, seed {}, {} of {} families differ{}",
        b.point,
        b.lower,
        b.upper,
        b.lower_percentile,
        b.upper_percentile,
        b.resamples,
        b.seed,
        b.differing_families,
        b.families,
        if b.insufficient_tail {
            " (tail unresolved)"
        } else {
            ""
        }
    );
    for reason in &e.verdict.failed {
        println!("gate not held: {reason}");
    }
    println!(
        "outcome: {}",
        serde_json::to_string(&e.verdict.outcome)
            .unwrap_or_default()
            .trim_matches('"')
    );
    0
}
