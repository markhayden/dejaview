//! `squint sets`: check a tree of evaluation sets, fetch a set's images by hash, seal a set and
//! verify its seal (task 3.1, docs/evalsets.md). Exit codes: 0 ok; 1 the set, its images or its
//! seal do not hold; 64 usage (no set at the path, a missing argument).

use std::path::{Path, PathBuf};

use clap::Subcommand;
use squint_eval::sets;

use crate::EXIT_USAGE;

const EXIT_FAILED: i32 = 1;

#[derive(clap::Args)]
pub struct SetsArgs {
    #[command(subcommand)]
    pub command: SetsCommand,
}

#[derive(Subcommand)]
pub enum SetsCommand {
    /// Check every set under a root (or one set directory): manifests, rules, jobs, images,
    /// families and labels.
    Check(CheckArgs),
    /// Place a set's images beside its manifest from its images_repo, refusing any hash mismatch.
    Fetch(SetArgs),
    /// Check a set and write seal.toml over its content, with the owner's attestation.
    Seal(SealArgs),
    /// Recompute a set's seal components and name any that changed.
    VerifySeal(SetArgs),
}

#[derive(clap::Args)]
pub struct CheckArgs {
    /// The evalsets root (`<use-case>/<set>/set.toml` below it) or one set directory.
    #[arg(default_value = "evalsets")]
    root: PathBuf,
}

#[derive(clap::Args)]
pub struct SetArgs {
    /// A set id (`<use-case>/<set>`, resolved under --root) or the path of a set directory.
    #[arg(long)]
    set: String,
    /// The evalsets root that set ids resolve under.
    #[arg(long, default_value = "evalsets")]
    root: PathBuf,
}

#[derive(clap::Args)]
pub struct SealArgs {
    #[command(flatten)]
    set: SetArgs,
    /// The owner's attestation that no judge or tool has seen the items.
    #[arg(long)]
    attest: String,
    /// For a blind set whose labels sit beside the manifest: the owner-storage directory the
    /// label file is moved to.
    #[arg(long)]
    labels_out: Option<PathBuf>,
}

impl SetArgs {
    /// The set directory: the path itself when it holds a manifest, otherwise the id under the root.
    fn dir(&self) -> Result<PathBuf, String> {
        let as_path = Path::new(&self.set);
        if as_path.join(sets::MANIFEST_FILE).is_file() {
            return Ok(as_path.to_path_buf());
        }
        let under_root = self.root.join(&self.set);
        if under_root.join(sets::MANIFEST_FILE).is_file() {
            return Ok(under_root);
        }
        Err(format!(
            "no set at {} or under {} ({} not found)",
            as_path.display(),
            self.root.display(),
            sets::MANIFEST_FILE
        ))
    }
}

pub fn run(args: &SetsArgs) -> i32 {
    match &args.command {
        SetsCommand::Check(a) => check(a),
        SetsCommand::Fetch(a) => fetch(a),
        SetsCommand::Seal(a) => seal(a),
        SetsCommand::VerifySeal(a) => verify_seal(a),
    }
}

fn check(args: &CheckArgs) -> i32 {
    if let Err(e) = sets::discover(&args.root) {
        eprintln!("squint: {e}");
        return EXIT_USAGE;
    }
    match sets::check(&args.root) {
        Ok(r) => {
            println!(
                "sets check ok: {} set{}, {} items, {} images under {}",
                r.sets,
                if r.sets == 1 { "" } else { "s" },
                r.items,
                r.images,
                args.root.display()
            );
            0
        }
        Err(e) => {
            eprintln!("squint: {e}");
            EXIT_FAILED
        }
    }
}

fn set_dir(args: &SetArgs) -> Result<PathBuf, i32> {
    args.dir().map_err(|e| {
        eprintln!("squint: {e}");
        EXIT_USAGE
    })
}

fn fetch(args: &SetArgs) -> i32 {
    let dir = match set_dir(args) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("squint: runtime: {e}");
            return EXIT_FAILED;
        }
    };
    match runtime.block_on(sets::fetch(&dir)) {
        Ok(r) => {
            println!(
                "sets fetch ok: {} fetched, {} present in {}",
                r.fetched,
                r.present,
                dir.display()
            );
            0
        }
        Err(e) => {
            eprintln!("squint: {e}");
            EXIT_FAILED
        }
    }
}

fn seal(args: &SealArgs) -> i32 {
    let dir = match set_dir(&args.set) {
        Ok(d) => d,
        Err(code) => return code,
    };
    match sets::seal(&dir, &args.attest, args.labels_out.as_deref()) {
        Ok(seal) => {
            let set = sets::Set::load(&dir).map(|s| s.manifest.set.id);
            println!(
                "sealed {} at {}: {}",
                set.unwrap_or_else(|_| dir.display().to_string()),
                seal.seal.created,
                seal.seal.sha256
            );
            0
        }
        Err(e) => {
            eprintln!("squint: {e}");
            EXIT_FAILED
        }
    }
}

fn verify_seal(args: &SetArgs) -> i32 {
    let dir = match set_dir(args) {
        Ok(d) => d,
        Err(code) => return code,
    };
    match sets::verify_seal(&dir) {
        Ok(()) => {
            println!("seal holds for {}", dir.display());
            0
        }
        Err(e) => {
            eprintln!("squint: {e}");
            EXIT_FAILED
        }
    }
}
