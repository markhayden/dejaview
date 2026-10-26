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
CI as `just deny-advisories`), `scripts/check-crate-graph.sh`, `scripts/test-hooks.sh` (the
credential scanner and the pre-commit hook, end to end in a scratch repository) and
`scripts/test-checks.sh` (the checks fail when they should).

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
fallback name is needed. Publication order and the registry install check are task 2.10a.

`cargo-dist` installs an executable named `dist` (run `dist --version`), used from task 2.10b.

## Running CI locally

`brew install act` once, then `just ci-local` runs the GitHub Actions workflow in a Linux
container on this machine, before anything is pushed. It validates the workflow file, installs
the pinned toolchain and tools as the runner would, and runs `just check` under GNU tools rather
than the macOS ones. GitHub's cache action and runner image specifics are the only parts it does
not reproduce.
