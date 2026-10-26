//! The `dejaview` binary: argument parsing and wiring only. All behaviour lives in the library
//! crates.

use clap::{Parser, Subcommand};

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("DEJAVIEW_GIT_SHA"),
    ")"
);

#[derive(Parser)]
#[command(name = "dejaview", version = VERSION, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Judge one image job and print the verdict (task 1.1 makes this real).
    Check(CheckArgs),
}

#[derive(clap::Args)]
struct CheckArgs {
    /// Path to the config file.
    #[arg(long)]
    config: Option<std::path::PathBuf>,
    /// Path to a job file.
    #[arg(long)]
    job: Option<std::path::PathBuf>,
    /// Scripted fake provider responses, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    provider_script: Option<std::path::PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None => {
            // `--version` and `--help` are handled by clap before we get here.
            eprintln!("dejaview {VERSION}: no command given; try --help");
            std::process::exit(64);
        }
        Some(Command::Check(_)) => {
            eprintln!("dejaview check: not implemented yet (PLAN.md task 1.1)");
            std::process::exit(64);
        }
    }
}
