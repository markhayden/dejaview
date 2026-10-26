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
check: fmt-check clippy test deny graph hooks-test checks-test

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --features dejaview/testkit -- -D warnings

test:
    cargo test --workspace
    cargo test --workspace --features dejaview/testkit

# Licences, bans and sources work offline; advisories need the network and run in CI (`just deny-advisories`).
deny:
    cargo deny check licenses bans sources

deny-advisories:
    cargo deny check advisories

graph:
    scripts/check-crate-graph.sh

# Run one filtered suite and fail if the filter selects no tests.
# Usage: just check-filter dejaview-core -- image frames
check-filter package *filters:
    scripts/check-filter.sh {{package}} {{filters}}

# Install the git hooks (task 0.2).
hooks:
    git config core.hooksPath scripts/hooks

# Prove the credential scanner behind the pre-commit hook accepts and rejects what it must.
hooks-test:
    scripts/test-hooks.sh

# Prove the repository checks (crate graph, filter) fail when they should.
checks-test:
    scripts/test-checks.sh

# Run the GitHub Actions CI workflow locally in Docker with act (brew install act). First run pulls
# the runner image and builds from scratch; later runs reuse the image.
# Pass a job name to run one job: `just ci-local rust`.
ci-local job="":
    act push {{ if job == "" { "" } else { "-j " + job } }} -P ubuntu-latest=catthehacker/ubuntu:act-latest --container-architecture linux/amd64 --rm
