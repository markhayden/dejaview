# dejaview developer recipes. See docs/bootstrap.md for the one manual step (installing just).

set shell := ["bash", "-euo", "pipefail", "-c"]

deny_version := "0.20.2"
dist_version := "0.32.0"

# Install the pinned helper tools (idempotent).
bootstrap:
    cargo install cargo-deny --version {{deny_version}} --locked
    cargo install cargo-dist --version {{dist_version}} --locked
    @command -v uv >/dev/null || echo "uv not found: install from https://docs.astral.sh/uv/ before M1 task 1.14"

# Everything a commit must pass: fmt, clippy and tests in both feature configurations, deny, graph.
check: fmt-check clippy test deny graph

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --features dejaview/testkit -- -D warnings

test:
    cargo test --workspace
    cargo test --workspace --features dejaview/testkit

deny:
    cargo deny check

graph:
    scripts/check-crate-graph.sh

# Run one filtered suite and fail if the filter selects no tests.
# Usage: just check-filter dejaview-core -- image frames
check-filter package *filters:
    scripts/check-filter.sh {{package}} {{filters}}

# Install the git hooks (task 0.2).
hooks:
    git config core.hooksPath scripts/hooks
