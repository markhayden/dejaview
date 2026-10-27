//! The `dejaview` binary: argument parsing and wiring only. All behaviour lives in the library
//! crates.

mod catalog;
mod eval;
mod report;

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
    /// Rebuild a report from a decision directory, or prune old decisions.
    Report(report::ReportArgs),
    /// Evaluation: the admission gate on a result table (the runner arrives with M3).
    Eval(eval::EvalArgs),
    /// The model catalog: limits, prices, capabilities and sources per model.
    Catalog(catalog::CatalogArgs),
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
    /// Install a tool pack's provisioning bundle: verify digests, unpack, `uv sync --frozen`.
    Setup(ToolsSetupArgs),
}

#[derive(clap::Args)]
struct ToolsSetupArgs {
    /// A bundle directory (`bundle.toml`, the wheel, the provisioning project and its lock).
    #[arg(long)]
    bundle: PathBuf,
    /// Where to install; defaults to `$XDG_DATA_HOME` or `~/.local/share`.
    #[arg(long)]
    data_home: Option<PathBuf>,
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
    /// Replay a recorded conversation (a directory of exchanges) instead of calling the provider;
    /// no credential is needed and nothing leaves the machine.
    #[arg(long, value_name = "DIR", conflicts_with = "record")]
    replay: Option<PathBuf>,
    /// Record every exchange with the provider into this directory, secrets stripped.
    #[arg(long, value_name = "DIR")]
    record: Option<PathBuf>,
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
        Some(Command::Tools(ToolsArgs {
            command: ToolsCommand::Setup(args),
        })) => tools_setup(&args),
        Some(Command::Report(args)) => report::run(&args),
        Some(Command::Eval(args)) => eval::run(&args),
        Some(Command::Catalog(args)) => catalog::run(&args),
    };
    std::process::exit(code);
}

fn check(args: &CheckArgs) -> i32 {
    let mut config = match Config::load_with_overrides(&args.config, &args.overrides) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    // Resolve-time checks against the catalog (profile, model, controls, image size, allowances)
    // and the consent binding. The scripted pair passes in testkit builds, and in a dry run,
    // where nothing is sent.
    if let Err(e) = dejaview_providers::catalog::Catalog::shipped().and_then(|c| {
        c.check_config(&config, cfg!(feature = "testkit") || args.dry_run)?;
        match c.profile(&config.provider.profile) {
            Some(profile) => {
                dejaview_providers::privacy::consent(&config, &config.provider.profile, profile)
                    .map(|_| ())
            }
            None => Ok(()),
        }
    }) {
        eprintln!("dejaview: {e}");
        return EXIT_USAGE;
    }
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
    // Every enabled tool and `submit_verdict` must have a wire form for the selected model, and
    // the set must fit the provider's limits, before any billable request.
    if let Err(e) = check_wire_schemas(&config, &tools) {
        eprintln!("dejaview: {e}");
        return EXIT_USAGE;
    }
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
    let provider: Box<dyn Provider> = match select_provider(args, &mut config) {
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
    // The handlers are registered here, before the run starts, so a signal that lands during the
    // run's first synchronous work is caught rather than taking the process down.
    let signals = {
        let _guard = runtime.enter();
        install_signal_handlers()
    };
    runtime.spawn(cancel_on_signal(signals, std::sync::Arc::clone(&cancel)));
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
            if o.cache.is_hit() {
                eprintln!(
                    "cache: hit (source: {})",
                    o.cache
                        .source
                        .as_deref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                );
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

/// The registered signal streams (unix) or nothing (elsewhere, where `ctrl_c` registers on await).
#[cfg(unix)]
type Signals = Option<(tokio::signal::unix::Signal, tokio::signal::unix::Signal)>;
#[cfg(not(unix))]
type Signals = ();

/// Registers SIGINT and SIGTERM with the runtime; must run inside a runtime context.
#[cfg(unix)]
fn install_signal_handlers() -> Signals {
    use tokio::signal::unix::{SignalKind, signal};
    match (
        signal(SignalKind::interrupt()),
        signal(SignalKind::terminate()),
    ) {
        (Ok(int), Ok(term)) => Some((int, term)),
        _ => None,
    }
}

#[cfg(not(unix))]
fn install_signal_handlers() -> Signals {}

/// SIGINT or SIGTERM cancels the run; it appends `cancelled` and exits 3.
async fn cancel_on_signal(signals: Signals, cancel: std::sync::Arc<Cancel>) {
    #[cfg(unix)]
    {
        match signals {
            Some((mut int, mut term)) => {
                tokio::select! {
                    _ = int.recv() => {}
                    _ = term.recv() => {}
                }
            }
            None => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = signals;
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

/// The provider for this run: the scripted fake (testkit builds, `--provider-script`), a replay of
/// a recorded conversation, or the live adapter for the configured profile and model over HTTPS,
/// optionally recording every exchange. The credential joins the redaction list before any run.
fn select_provider(args: &CheckArgs, config: &mut Config) -> Result<Box<dyn Provider>, String> {
    use std::sync::Arc;

    use dejaview_providers::anthropic::AnthropicProvider;
    use dejaview_providers::catalog::{Catalog, FAKE_PROFILE};
    use dejaview_providers::http::{Client, Transport};
    use dejaview_providers::privacy::credential;
    use dejaview_providers::replay::{Player, Recorder};
    #[cfg(feature = "testkit")]
    if let Some(path) = &args.provider_script {
        return dejaview_testkit::ScriptedProvider::from_path(path)
            .map(|p| Box::new(p) as Box<dyn Provider>)
            .map_err(|e| e.to_string());
    }
    if config.provider.profile == FAKE_PROFILE {
        return Err("the fake profile runs only with --provider-script in a testkit build".into());
    }
    let catalog = Catalog::shipped().map_err(|e| e.to_string())?;
    let profile = catalog.profile(&config.provider.profile).ok_or_else(|| {
        format!(
            "profile {:?} is not in the catalog",
            config.provider.profile
        )
    })?;
    let model = catalog
        .model(&config.provider.model)
        .ok_or_else(|| format!("model {:?} is not in the catalog", config.provider.model))?;
    let (cred, transport): (_, Arc<dyn Transport>) = match &args.replay {
        Some(dir) => {
            let cred =
                credential(profile, &|_| Some("replay".into())).map_err(|e| e.to_string())?;
            let player =
                Player::load(dir, AnthropicProvider::volatile()).map_err(|e| e.to_string())?;
            (cred, Arc::new(player))
        }
        None => {
            let cred =
                credential(profile, &|name| std::env::var(name).ok()).map_err(|e| e.to_string())?;
            config.secrets.push(cred.expose().to_string());
            let client = Client::new(REQUEST_TIMEOUT).map_err(|e| e.to_string())?;
            let base: Arc<dyn Transport> = Arc::new(client);
            match &args.record {
                Some(dir) => {
                    let recorder = Recorder::new(
                        base,
                        dir,
                        vec![cred.expose().to_string()],
                        AnthropicProvider::volatile(),
                    )
                    .map_err(|e| e.to_string())?;
                    (cred, Arc::new(recorder))
                }
                None => (cred, base),
            }
        }
    };
    match model.adapter.as_str() {
        "anthropic-messages" => AnthropicProvider::new(
            config,
            &config.provider.profile,
            profile,
            model,
            cred,
            transport,
        )
        .map(|p| Box::new(p) as Box<dyn Provider>)
        .map_err(|e| e.to_string()),
        other => Err(format!(
            "adapter {other} is not implemented yet (Gemini arrives with task 2.6a, OpenAI with 2.7a)"
        )),
    }
}

/// The per-request HTTP timeout: long enough for a high-effort turn, bounded.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

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

/// The wire transformation of every tool definition for the configured model (SPEC "Wire
/// schemas"). The scripted pair has no adapter and nothing to check.
fn check_wire_schemas(
    config: &Config,
    tools: &dejaview_core::tool::ToolRegistry,
) -> dejaview_core::Result<()> {
    use dejaview_providers::catalog::{Catalog, FAKE_MODEL};
    use dejaview_providers::wire::{Adapter, check_tool_set};
    if config.provider.model == FAKE_MODEL {
        return Ok(());
    }
    let catalog = Catalog::shipped()?;
    let Some(model) = catalog.model(&config.provider.model) else {
        return Ok(());
    };
    let Some(adapter) = Adapter::parse(&model.adapter) else {
        return Err(dejaview_core::Error::Config(format!(
            "catalog: model {:?} names an adapter this build lacks: {}",
            config.provider.model, model.adapter
        )));
    };
    let mut definitions: Vec<dejaview_core::provider::ToolDefinition> =
        tools.manifests().map(|m| m.tool_definition()).collect();
    definitions.push(dejaview_core::submission::submit_verdict_tool());
    check_tool_set(adapter, &model.capabilities, &definitions).map(|_| ())
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
    let mut known = known_manifests();
    let packs = match connect_packs(&config) {
        Ok(p) => p,
        Err(message) => {
            eprintln!("dejaview: {message}");
            return EXIT_USAGE;
        }
    };
    for pack in &packs {
        known.extend(pack.manifests().iter().cloned());
    }
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
    // Packs first; a built-in or scripted tool of the same name yields to the pack's.
    let mut available: Vec<Box<dyn Tool>> = Vec::new();
    for pack in connect_packs(config)? {
        available.extend(pack.tools());
    }
    for tool in available_tools(args)? {
        if !available
            .iter()
            .any(|t| t.manifest().name == tool.manifest().name)
        {
            available.push(tool);
        }
    }
    ToolRegistry::new(available, &config.tools.enabled, &config.tools.order)
        .map_err(|e| e.to_string())
}

fn tools_setup(args: &ToolsSetupArgs) -> i32 {
    use dejaview_tools::provision::{Bundle, data_home};
    let home = args.data_home.clone().unwrap_or_else(data_home);
    let bundle = match Bundle::load(&args.bundle) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("dejaview: {e}");
            return EXIT_USAGE;
        }
    };
    match bundle.install(&home) {
        Ok(installed) => {
            println!(
                "installed {} {} into {}",
                installed.header.package,
                installed.header.version,
                installed.dir.display()
            );
            println!("executable: {}", installed.executable.display());
            println!("lock digest: {}", installed.lock_digest);
            0
        }
        Err(e) => {
            eprintln!("dejaview: {e}");
            EXIT_USAGE
        }
    }
}

/// Connects every pack the config names: by `command`, or by `package` and `version` from an
/// installed bundle.
fn connect_packs(
    config: &Config,
) -> Result<Vec<std::sync::Arc<dejaview_tools::mcp::Pack>>, String> {
    use dejaview_tools::mcp::{Pack, Spawn, Timeouts};
    use dejaview_tools::provision::{Installed, data_home};
    use std::time::Duration;
    let mut packs = Vec::new();
    for (name, pack) in &config.tools.packs {
        let spawn = match (&pack.command, &pack.package, &pack.version) {
            (Some(command), _, _) => Spawn {
                command: command.clone(),
                args: pack.args.clone(),
                declared_env: pack.env.clone(),
                cwd: None,
                extra_env: Vec::new(),
            },
            (None, Some(package), Some(version)) => {
                Installed::locate(&data_home(), package, version)
                    .map_err(|e| e.to_string())?
                    .spawn(pack.env.clone())
            }
            _ => continue,
        };
        let timeouts = Timeouts {
            startup: Duration::from_secs(pack.startup_timeout_s),
            call: Duration::from_secs(config.budget.tool_timeout_s),
            shutdown_grace: Duration::from_secs(5),
        };
        packs.push(
            Pack::connect(name, spawn, timeouts, std::env::vars()).map_err(|e| e.to_string())?,
        );
    }
    Ok(packs)
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
