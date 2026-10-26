//! The `dejaview` binary: argument parsing and wiring only. All behaviour lives in the library
//! crates.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use dejaview_core::config::Config;
use dejaview_core::job::Job;
use dejaview_core::provider::Provider;
use dejaview_core::run::{Cancel, Services};
use dejaview_core::tool::{Tool, ToolRegistry};

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

// Parsed once per process; the size difference between subcommands is irrelevant here.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum Command {
    /// Judge one image job and print the verdict.
    Check(CheckArgs),
    /// Inspect rules.
    Rules(RulesArgs),
    /// Inspect tools.
    Tools(ToolsArgs),
}

#[derive(clap::Args)]
struct ToolsArgs {
    #[command(subcommand)]
    command: ToolsCommand,
}

#[derive(Subcommand)]
enum ToolsCommand {
    /// Print the manifests of the enabled tools as the model will see them.
    List(ToolsListArgs),
}

#[derive(clap::Args)]
struct ToolsListArgs {
    /// Path to the config file; `[tools] enabled` names the tools.
    #[arg(long)]
    config: PathBuf,
    /// Print JSON instead of the text form.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct RulesArgs {
    #[command(subcommand)]
    command: RulesCommand,
}

#[derive(Subcommand)]
enum RulesCommand {
    /// Print the merged obligations for a config (and optionally a job) with provenance.
    Show(RulesShowArgs),
}

#[derive(clap::Args)]
struct RulesShowArgs {
    /// Path to the config file; `[rules]` names the base, profile and extra files.
    #[arg(long)]
    config: PathBuf,
    /// Optional job file, so scope, evidence and required roles are checked against it.
    #[arg(long)]
    job: Option<PathBuf>,
    /// Print JSON instead of the table.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct CheckArgs {
    /// Path to the config file.
    #[arg(long)]
    config: PathBuf,
    /// Override one config value: `--set budget.tool_turns=2` (repeatable).
    #[arg(long = "set", value_name = "PATH=VALUE")]
    overrides: Vec<String>,
    /// Path to a job file. Relative image paths in it resolve against the current directory.
    #[arg(long, conflicts_with_all = ["candidate", "source", "reference", "brief", "use_case"])]
    job: Option<PathBuf>,
    /// Shorthand job: the candidate image.
    #[arg(long, requires = "use_case")]
    candidate: Option<PathBuf>,
    /// Shorthand job: the source image the candidate was made from.
    #[arg(long, requires = "candidate")]
    source: Option<PathBuf>,
    /// Shorthand job: a reference image as `label=path` (repeatable); its id becomes `ref:<label>`.
    #[arg(long = "ref", value_name = "LABEL=PATH", requires = "candidate")]
    reference: Vec<String>,
    /// Shorthand job: the free-text brief.
    #[arg(long, requires = "candidate")]
    brief: Option<String>,
    /// Shorthand job: the use case, for example `source-fidelity`.
    #[arg(long, requires = "candidate")]
    use_case: Option<String>,
    /// Output directory for run directories; overrides `[output] dir`.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Resolve the rules and print the Obligations section, the instances and the checklist
    /// without calling any provider.
    #[arg(long)]
    dry_run: bool,
    /// Scripted fake provider responses, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    provider_script: Option<PathBuf>,
    /// Scripted envelopes for the fake `fake_regions` tool, for tests and debugging only.
    #[cfg(feature = "testkit")]
    #[arg(long)]
    tool_script: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        None => {
            eprintln!("dejaview {VERSION}: no command given; try --help");
            EXIT_USAGE
        }
        Some(Command::Check(args)) => check(&args),
        Some(Command::Rules(RulesArgs {
            command: RulesCommand::Show(args),
        })) => rules_show(&args),
        Some(Command::Tools(ToolsArgs {
            command: ToolsCommand::List(args),
        })) => tools_list(&args),
    };
    std::process::exit(code);
}

fn check(args: &CheckArgs) -> i32 {
    let config = match Config::load_with_overrides(&args.config, &args.overrides) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let job = match load_job(args) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let tools = match build_tools(args, &config) {
        Ok(t) => t,
        Err(message) => {
            eprintln!("dejaview: {message}");
            return EXIT_USAGE;
        }
    };
    if args.dry_run {
        return match dejaview_core::run::resolve_rules(&config, &job, Path::new("."), &tools) {
            Ok(p) => {
                println!("{}", p.system_prompt);
                println!("Instances:");
                for i in &p.resolved.instances {
                    println!(
                        "  {} kind={:?} matters={} hidden_ok={} evidence={:?} kinds={:?} [{}]",
                        i.id(),
                        i.kind,
                        i.matters,
                        i.hidden_ok,
                        i.evidence,
                        i.evidence_kinds,
                        i.provenance
                    );
                }
                println!("Checklist:");
                for c in &p.checklist.items {
                    println!("  {} <- {:?}", c.id, c.instance_ids);
                }
                for w in &p.summary.warnings {
                    eprintln!("warning: {w}");
                }
                0
            }
            Err(e) => {
                eprintln!("dejaview: {e}");
                EXIT_USAGE
            }
        };
    }
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
    let cancel = std::sync::Arc::new(Cancel::new());
    runtime.spawn(cancel_on_signal(std::sync::Arc::clone(&cancel)));
    let services = Services::new(provider.as_ref(), &tools).with_cancel(&cancel);
    let outcome = runtime.block_on(dejaview_core::run::check(
        &config, &job, job_base, &services, &out_dir, VERSION,
    ));
    match outcome {
        Ok(o) => {
            match (&o.decision.verdict, &o.error) {
                (Some(v), _) => println!(
                    "{}",
                    serde_json::to_string(v)
                        .unwrap_or_default()
                        .trim_matches('"')
                ),
                (None, Some(e)) => eprintln!("error: {}", redacted(e, &config)),
                (None, None) => eprintln!("error: run ended without a verdict or an error"),
            }
            eprintln!("decision: {}", o.dir.display());
            o.exit_code
        }
        Err(e) => {
            eprintln!("dejaview: {}", redacted(&e.to_string(), &config));
            dejaview_core::run::EXIT_ERROR
        }
    }
}

/// SIGINT or SIGTERM cancels the run; it appends `cancelled` and exits 3.
async fn cancel_on_signal(cancel: std::sync::Arc<Cancel>) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut term) = signal(SignalKind::terminate()) else {
            let _ = tokio::signal::ctrl_c().await;
            cancel.cancel();
            return;
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    cancel.cancel();
}

/// Error text never carries an expanded secret.
fn redacted(text: &str, config: &Config) -> String {
    let mut out = text.to_string();
    for s in &config.secrets {
        if !s.is_empty() {
            out = out.replace(s, "[redacted]");
        }
    }
    out
}

fn load_job(args: &CheckArgs) -> Result<Job, String> {
    match (&args.job, &args.candidate) {
        (Some(path), None) => Job::load(path).map_err(|e| e.to_string()),
        (None, Some(candidate)) => {
            let use_case = args.use_case.as_deref().unwrap_or("source-fidelity");
            let mut refs = Vec::new();
            for r in &args.reference {
                let (label, path) = r
                    .split_once('=')
                    .ok_or_else(|| format!("--ref {r:?} must be label=path"))?;
                refs.push((label.to_string(), PathBuf::from(path)));
            }
            Job::from_shorthand(
                use_case,
                candidate.clone(),
                args.source.clone(),
                &refs,
                args.brief.clone(),
            )
            .map_err(|e| e.to_string())
        }
        (None, None) => Err("pass --job <file> or the shorthand --candidate … --use-case …".into()),
        (Some(_), Some(_)) => Err("--job and --candidate are exclusive".into()),
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

fn rules_show(args: &RulesShowArgs) -> i32 {
    use dejaview_core::obligations::{MergeInput, RulesFile, merge};
    let config = match Config::load(&args.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let Some(rules) = &config.rules else {
        eprintln!("dejaview: the config has no [rules] section");
        return EXIT_USAGE;
    };
    let job = match &args.job {
        Some(path) => match Job::load(path) {
            Ok(j) => Some(j),
            Err(e) => {
                eprintln!("dejaview: {e}");
                return EXIT_USAGE;
            }
        },
        None => None,
    };
    let mut extra = Vec::new();
    let mut names = Vec::new();
    for path in &rules.extra {
        match RulesFile::load(path) {
            Ok(f) => {
                extra.push(f);
                names.push(path.display().to_string());
            }
            Err(e) => {
                eprintln!("dejaview: {e}");
                return EXIT_USAGE;
            }
        }
    }
    let merged = merge(&MergeInput {
        base: &rules.base,
        profile: rules.profile.as_deref(),
        extra: &extra,
        extra_names: &names,
        job: job.as_ref(),
        enabled_tools: &config.tools.enabled,
    });
    match merged {
        Ok(m) => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&m).unwrap_or_default());
            } else {
                println!(
                    "use case: {}  base: {}  profile: {}",
                    m.use_case,
                    m.base,
                    m.profile.as_deref().unwrap_or("-")
                );
                for o in &m.obligations {
                    let ob = &o.obligation;
                    println!(
                        "{:<5} {:<10} matters={:<5} hidden_ok={:<5} [{}] {}",
                        ob.id,
                        format!("{:?}", ob.kind).to_lowercase(),
                        ob.matters(),
                        ob.hidden_ok,
                        o.provenance,
                        ob.text
                    );
                }
                for w in &m.warnings {
                    eprintln!("warning: {w}");
                }
            }
            0
        }
        Err(e) => {
            eprintln!("dejaview: {e}");
            EXIT_USAGE
        }
    }
}

/// The tools this build knows. Built-ins arrive in M4; the fake tool exists behind `testkit`.
fn known_manifests() -> Vec<dejaview_core::tool::Manifest> {
    #[cfg(feature = "testkit")]
    {
        vec![
            dejaview_core::tool::Manifest::from_toml(dejaview_testkit::tool::FAKE_REGIONS_MANIFEST)
                .expect("fake_regions manifest is valid"),
        ]
    }
    #[cfg(not(feature = "testkit"))]
    {
        Vec::new()
    }
}

fn tools_list(args: &ToolsListArgs) -> i32 {
    let config = match Config::load(&args.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    let known = known_manifests();
    let mut listed = Vec::new();
    for name in &config.tools.enabled {
        match known.iter().find(|m| &m.name == name) {
            Some(m) => listed.push(m.clone()),
            None => {
                eprintln!("dejaview: tool {name:?} is enabled but not known to this build");
                return EXIT_USAGE;
            }
        }
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&listed).unwrap_or_default()
        );
        return 0;
    }
    for m in &listed {
        println!(
            "{} v{} ({:?}, ~{}s)",
            m.name, m.version, m.cost.tier, m.cost.seconds
        );
        println!("  purpose: {}", m.purpose);
        println!("  when to use: {}", m.when_to_use);
        println!("  when not to use: {}", m.when_not_to_use);
        println!("  output: {}", m.output_guidance);
        if let Some(k) = &m.evidence_kind {
            println!("  evidence kind: {k}");
        }
        if !m.requires.is_empty() {
            println!("  requires: {}", m.requires.join(", "));
        }
        println!(
            "  model schema: {}",
            serde_json::to_string(&m.model_schema).unwrap_or_default()
        );
    }
    0
}

/// The tools available to this build, then narrowed to the config's enabled list.
fn build_tools(args: &CheckArgs, config: &Config) -> Result<ToolRegistry, String> {
    let available: Vec<Box<dyn Tool>> = available_tools(args)?;
    ToolRegistry::new(available, &config.tools.enabled, &config.tools.order)
        .map_err(|e| e.to_string())
}

#[cfg(feature = "testkit")]
fn available_tools(args: &CheckArgs) -> Result<Vec<Box<dyn Tool>>, String> {
    let mut out: Vec<Box<dyn Tool>> = Vec::new();
    match &args.tool_script {
        Some(path) => out.push(Box::new(
            dejaview_testkit::ScriptedTool::fake_regions_from_path(path)
                .map_err(|e| e.to_string())?,
        )),
        None => out.push(Box::new(dejaview_testkit::ScriptedTool::fake_regions(
            Vec::new(),
        ))),
    }
    Ok(out)
}

#[cfg(not(feature = "testkit"))]
fn available_tools(_args: &CheckArgs) -> Result<Vec<Box<dyn Tool>>, String> {
    Ok(Vec::new())
}
