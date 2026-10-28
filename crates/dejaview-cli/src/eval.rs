//! `dejaview eval`: the matrix runner (3.4b) and the gate on a statistics fixture (1.15); the
//! estimate, the report and the gate lifecycle arrive with the rest of M3.

use std::path::PathBuf;

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
    /// Apply the admission gate to a synthetic result table and print the interval and outcome.
    Gate(GateArgs),
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
    let matrix_text = match std::fs::read_to_string(&args.matrix) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("dejaview: cannot read {}: {e}", args.matrix.display());
            return EXIT_USAGE;
        }
    };
    let matrix = match Matrix::parse(&matrix_text) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("dejaview: {}: {e}", args.matrix.display());
            return EXIT_USAGE;
        }
    };
    let config_text = match std::fs::read_to_string(&matrix.matrix.config) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "dejaview: cannot read the matrix's config {}: {e}",
                matrix.matrix.config.display()
            );
            return EXIT_USAGE;
        }
    };
    let admissions_path = args.admissions.clone().or_else(|| {
        let p = args.evalsets_root.join("admissions.toml");
        p.exists().then_some(p)
    });
    let admissions = match admissions_path.as_deref().map(Admissions::load) {
        Some(Ok(a)) => Some(a),
        Some(Err(e)) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
        None => None,
    };
    let plan = Plan {
        eval_dir: args.out.clone(),
        matrix,
        matrix_text,
        config_text,
        evalsets_root: args.evalsets_root.clone(),
        admissions,
        harness_version: VERSION.to_string(),
        allow_fake: cfg!(feature = "testkit"),
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
