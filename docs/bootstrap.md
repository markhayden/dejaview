# Bootstrap

One manual step, then everything else is a `just` recipe.

```
cargo install just --version 1.59.0 --locked    # or: brew install just
just bootstrap                                  # pinned cargo-deny and cargo-dist
just check                                      # what every commit must pass
```

`just check` runs `cargo fmt --all -- --check`, Clippy with `-D warnings` in both feature
configurations (default, and `--features dejaview/testkit`), `cargo test --workspace` in both
configurations, `cargo deny check licenses bans sources` (advisories need the network and run in
CI as `just deny-advisories`), `scripts/check-crate-graph.sh`, `scripts/check-packaging.sh`
(every asset a crate embeds sits inside that crate and in the file list `cargo package` ships),
`scripts/test-hooks.sh` (the credential scanner and the pre-commit hook, end to end in a scratch
repository) and `scripts/test-checks.sh` (the checks fail when they should).

`just hooks` installs the pre-commit hook from `scripts/hooks/`. It scans every staged blob for
credential-shaped values first, then runs fmt and Clippy. Only the exact synthetic values in
`scripts/synthetic-tokens.txt` are allowed, and each must contain `SYNTHETIC`.

The `testkit` feature of the `dejaview` package enables the scripted fake provider and the
`--provider-script` flag for tests and debugging. Release builds never enable it.

Toolchain: `rust-toolchain.toml` pins an exact stable version (bumped deliberately, since `just
check` runs Clippy with `-D warnings`) with `clippy` and `rustfmt`; rustup installs them on first use. `uv` is needed from M1 task 1.14 onward for the Python tool pack.

## Package names

The six package names (`dejaview`, `dejaview-core`, `dejaview-providers`, `dejaview-tools`,
`dejaview-eval`, `dejaview-testkit`) were all unregistered on crates.io on 2026-10-26, so no
fallback name is needed.

## Publishing

The shipped rule sets, prompt template and model catalog live inside the crates that embed them
(`crates/dejaview-core/rules/`, `crates/dejaview-core/prompts/`,
`crates/dejaview-providers/catalog.toml`), so a published crate builds from its own files. The core
crate's dev-dependency on the testkit is path-only (no version): `cargo publish` drops it, which
breaks the cycle between the two crates.

- `just publish-dry-run` (`scripts/publish.sh --dry-run`) packages and verifies every crate as
  `cargo publish --workspace` would, uploading nothing. The real publication is the owner's:
  `scripts/publish.sh` from a clean checkout whose `HEAD` carries the tag `v<version>`, after
  `cargo login`. Cargo orders the crates itself: core, testkit, providers, tools, eval, dejaview.
- `just registry-install-test` (`scripts/registry-install-test.sh`) installs `dejaview` from the
  packaged `.crate` files in a scratch directory outside the repository, with the sibling crates
  supplied from the extracted packages the way crates.io would supply them, then runs the offline
  smoke.
- `just smoke <binary>` (`scripts/offline-smoke.sh`) is that smoke on any built binary:
  `--version`, `tools list`, `rules show`, and a `check` without a credential that must stop with
  `auth:` and exit 64 before anything leaves the machine.

The `package` workflow runs all three on every push. `dejaview --version` prints the short commit;
a packaged crate has no repository, so the build script reads the commit `cargo package` records
in `.cargo_vcs_info.json` instead.

`cargo-dist` installs an executable named `dist` (run `dist --version`), used from task 2.10b.

## Running CI locally

`brew install act` once, then `just ci-local` runs the GitHub Actions workflow in a Linux
container on this machine, before anything is pushed. It validates the workflow file, installs
the pinned toolchain and tools as the runner would, and runs `just check` under GNU tools rather
than the macOS ones. GitHub's cache action and runner image specifics are the only parts it does
not reproduce.

## Credentials

dejaview reads API keys from the environment only and never writes them to disk. Keep them in a
`.env` file (git ignores it; `.env.example` lists the names) and load it into the shell before a
live run:

```sh
set -a; source .env; set +a
```

The `DEJAVIEW_*_ACCOUNT` entries feed the `[privacy] account` attestation through `${…}` expansion
in the config, for example `account = "anthropic:${DEJAVIEW_ANTHROPIC_ACCOUNT}"`.

