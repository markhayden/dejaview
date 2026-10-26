//! `dejaview eval`: the gate on a statistics fixture (task 1.15); the matrix runner, estimate and
//! report arrive with M3.

use std::path::PathBuf;

use clap::Subcommand;
use dejaview_eval::fixture::StatsFixture;

use crate::EXIT_USAGE;

#[derive(clap::Args)]
pub struct EvalArgs {
    #[command(subcommand)]
    pub command: EvalCommand,
}

#[derive(Subcommand)]
pub enum EvalCommand {
    /// Apply the admission gate to a synthetic result table and print the interval and outcome.
    Gate(GateArgs),
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
