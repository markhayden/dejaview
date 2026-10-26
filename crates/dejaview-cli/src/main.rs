//! The `dejaview` binary: argument parsing and wiring only. All behaviour lives in the library
//! crates.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use dejaview_core::config::Config;
use dejaview_core::job::Job;
use dejaview_core::provider::Provider;

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("DEJAVIEW_GIT_SHA"),
    ")"
);

/// Usage error: bad arguments or a configuration the binary cannot act on.
const EXIT_USAGE: i32 = 64;

#[derive(Parser)]
#[command(name = "dejaview", version = VERSION, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Judge one image job and print the verdict.
    Check(CheckArgs),
}

#[derive(clap::Args)]
struct CheckArgs {
    /// Path to the config file.
    #[arg(long)]
    config: PathBuf,
    /// Path to a job file. Relative image paths in it resolve against the current directory.
    #[arg(long)]
    job: PathBuf,
    /// Output directory for run directories; overrides `[output] dir`.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Scripted fake provider responses, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    provider_script: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        None => {
            eprintln!("dejaview {VERSION}: no command given; try --help");
            EXIT_USAGE
        }
        Some(Command::Check(args)) => check(&args),
    };
    std::process::exit(code);
}

fn check(args: &CheckArgs) -> i32 {
    let config = match Config::load(&args.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let job = match Job::load(&args.job) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let provider: Box<dyn Provider> = match select_provider(args) {
        Ok(p) => p,
        Err(message) => {
            eprintln!("dejaview: {message}");
            return EXIT_USAGE;
        }
    };
    let out_dir = args
        .out
        .clone()
        .unwrap_or_else(|| config.output.dir.clone());
    let job_base = Path::new(".");

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("dejaview: cannot start the runtime: {e}");
            return EXIT_USAGE;
        }
    };
    let outcome = runtime.block_on(dejaview_core::run::check(
        &config,
        &job,
        job_base,
        provider.as_ref(),
        &out_dir,
        VERSION,
    ));
    match outcome {
        Ok(o) => {
            match (&o.verdict, &o.error) {
                (Some(v), _) => println!(
                    "{}",
                    serde_json::to_string(v)
                        .unwrap_or_default()
                        .trim_matches('"')
                ),
                (None, Some(e)) => eprintln!("error: {e}"),
                (None, None) => eprintln!("error: run ended without a verdict or an error"),
            }
            eprintln!("run: {}", o.run_dir.display());
            o.exit_code
        }
        Err(e) => {
            eprintln!("dejaview: {e}");
            dejaview_core::run::EXIT_ERROR
        }
    }
}

#[cfg(feature = "testkit")]
fn select_provider(args: &CheckArgs) -> Result<Box<dyn Provider>, String> {
    match &args.provider_script {
        Some(path) => dejaview_testkit::ScriptedProvider::from_path(path)
            .map(|p| Box::new(p) as Box<dyn Provider>)
            .map_err(|e| e.to_string()),
        None => Err(
            "no provider adapter is available yet (adapters arrive in M2); pass --provider-script"
                .into(),
        ),
    }
}

#[cfg(not(feature = "testkit"))]
fn select_provider(_args: &CheckArgs) -> Result<Box<dyn Provider>, String> {
    Err("no provider adapter is available yet (adapters arrive in M2)".into())
}
