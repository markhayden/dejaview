//! `squint catalog`: show a model's catalog entry, or list the catalog.

use clap::Subcommand;
use squint_providers::catalog::Catalog;

use crate::EXIT_USAGE;

#[derive(clap::Args)]
pub struct CatalogArgs {
    #[command(subcommand)]
    pub command: CatalogCommand,
}

#[derive(Subcommand)]
pub enum CatalogCommand {
    /// Print one model's entry: limits, prices, image rules, capabilities, controls, sources.
    Show(ShowArgs),
    /// List every model with its adapter and prices.
    List(ListArgs),
}

#[derive(clap::Args)]
pub struct ShowArgs {
    /// A model id, for example `claude-opus-5-5`.
    pub model: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args)]
pub struct ListArgs {
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: &CatalogArgs) -> i32 {
    let catalog = match Catalog::shipped() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("squint: {e}");
            return EXIT_USAGE;
        }
    };
    match &args.command {
        CatalogCommand::Show(a) => show(&catalog, a),
        CatalogCommand::List(a) => list(&catalog, a),
    }
}

fn show(catalog: &Catalog, args: &ShowArgs) -> i32 {
    let Some(m) = catalog.model(&args.model) else {
        eprintln!(
            "squint: model {:?} is not in the catalog; `squint catalog list` names the shipped models",
            args.model
        );
        return EXIT_USAGE;
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(m).unwrap_or_default());
        return 0;
    }
    println!("{} ({}) via {}", args.model, m.display, m.adapter);
    println!(
        "  context {} tokens, output up to {} tokens, verified {}",
        m.context_tokens, m.max_output_tokens, m.verified
    );
    let p = &m.prices;
    println!(
        "  prices per million tokens: input ${:.2}, output ${:.2}, cached input ${:.3}{}",
        p.input_per_million,
        p.output_per_million,
        p.cached_input_per_million,
        p.long_prompt
            .as_ref()
            .map(|lp| format!(
                "; over {} tokens: input ${:.2}, output ${:.2}",
                lp.threshold_tokens, lp.input_per_million, lp.output_per_million
            ))
            .unwrap_or_default()
    );
    let im = &m.images;
    println!(
        "  images: {} formats, native long edge {} px, up to {} per request, {} tokens per image at most{}",
        im.formats.len(),
        im.native_long_edge_px,
        im.max_per_request,
        im.tokens.max_tokens_per_image(),
        im.detail_control
            .as_ref()
            .map(|c| format!(", detail control `{c}`"))
            .unwrap_or_default()
    );
    let c = &m.capabilities;
    println!(
        "  capabilities: forced tool choice {}, strict tools {}, call ids {}, parallel tools {}, system messages in history {}, sampling {}",
        yes(c.forced_tool_choice),
        yes(c.strict_tools),
        yes(c.function_call_ids),
        yes(c.parallel_tools),
        yes(c.system_messages_in_history),
        yes(c.sampling)
    );
    println!("  final turn: {}", c.final_turn.join(", "));
    for (name, control) in &m.controls {
        println!(
            "  control {name}: {}{}",
            control.values.join(" | "),
            control
                .default
                .as_ref()
                .map(|d| format!(" (default {d})"))
                .unwrap_or_default()
        );
    }
    println!(
        "  control final_turn: {}",
        m.capabilities.final_turn.join(" | ")
    );
    if m.betas.contains_key("thinking_binding") {
        println!(
            "  control prefix_mismatch_behavior: error | drop_block (beta {})",
            m.betas["thinking_binding"]
        );
    }
    if !m.unverified.is_empty() {
        println!("  unverified: {}", m.unverified.join(", "));
    }
    println!("  sources:");
    for s in &m.sources {
        println!("    {s}");
    }
    0
}

fn list(catalog: &Catalog, args: &ListArgs) -> i32 {
    if args.json {
        let rows: Vec<serde_json::Value> = catalog
            .models
            .iter()
            .map(|(id, m)| {
                serde_json::json!({
                    "model": id, "adapter": m.adapter, "vendor": m.vendor,
                    "input_per_million": m.prices.input_per_million,
                    "output_per_million": m.prices.output_per_million,
                    "verified": m.verified,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).unwrap_or_default()
        );
        return 0;
    }
    for (id, m) in &catalog.models {
        println!(
            "{id:<20} {:<24} ${:>6.2} in  ${:>6.2} out  verified {}",
            m.adapter, m.prices.input_per_million, m.prices.output_per_million, m.verified
        );
    }
    println!(
        "profiles: {}",
        catalog
            .profiles
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    0
}

fn yes(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}
