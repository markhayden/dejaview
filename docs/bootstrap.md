# Bootstrap

One manual step, then everything else is a `just` recipe.

```
cargo install just --version 1.59.0 --locked    # or: brew install just
just bootstrap                                  # pinned cargo-deny and cargo-dist
just check                                      # what every commit must pass
```

`just check` runs `cargo fmt --all -- --check`, Clippy with `-D warnings` in both feature
configurations (default, and `--features dejaview/testkit`), `cargo test --workspace` in both
configurations, `cargo deny check`, and `scripts/check-crate-graph.sh`.

The `testkit` feature of the `dejaview` package enables the scripted fake provider and the
`--provider-script` flag for tests and debugging. Release builds never enable it.

Toolchain: `rust-toolchain.toml` pins the stable channel with `clippy` and `rustfmt`; rustup
installs them on first use. `uv` is needed from M1 task 1.14 onward for the Python tool pack.

## Package names

The six package names (`dejaview`, `dejaview-core`, `dejaview-providers`, `dejaview-tools`,
`dejaview-eval`, `dejaview-testkit`) were all unregistered on crates.io on 2026-10-26, so no
fallback name is needed. Publication order and the registry install check are task 2.10a.
