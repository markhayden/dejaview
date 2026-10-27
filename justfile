# dejaview developer recipes. See docs/bootstrap.md for the one manual step (installing just).

set shell := ["bash", "-euo", "pipefail", "-c"]

deny_version := "0.20.2"
dist_version := "0.32.0"

# Install the pinned helper tools (idempotent).
bootstrap:
    cargo install cargo-deny --version {{deny_version}} --locked
    cargo install cargo-dist --version {{dist_version}} --locked
    @command -v uv >/dev/null || echo "uv not found: install from https://docs.astral.sh/uv/ (the Python tool pack needs it)"

# Everything a commit must pass: fmt, clippy and tests in both feature configurations, deny, graph.
check: fmt-check clippy test deny graph packaging hooks-test checks-test

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

# Every embedded asset sits inside its crate and in the file list `cargo package` ships.
packaging:
    scripts/check-packaging.sh

# Package and verify every crate as `cargo publish` would, without uploading (network for the index).
publish-dry-run:
    scripts/publish.sh --dry-run

# Install `dejaview` from the packaged .crate files outside the repository and run the offline smoke.
registry-install-test:
    scripts/registry-install-test.sh

# The offline smoke against any built binary, for example `just smoke target/release/dejaview`.
smoke binary="target/debug/dejaview":
    scripts/offline-smoke.sh {{binary}}

# Run one filtered suite and fail if the filter selects no tests.
# Usage: just check-filter dejaview-core -- image frames
check-filter package *filters:
    scripts/check-filter.sh {{package}} {{filters}}

# The Python tool pack: lint, format check and tests (needs uv; network on the first `uv sync`).
python-check:
    cd tools/python && uv sync --frozen && uv run --frozen --offline --no-sync ruff check . && uv run --frozen --offline --no-sync ruff format --check . && uv run --frozen --offline --no-sync pytest -q

# Build the pack's provisioning bundle into tools/python/dist/bundle-<version>/.
bundle:
    scripts/build-bundle.sh

# Install the built bundle into a clean directory and call the pack through MCP (needs uv).
provisioning-test: bundle
    cargo test -p dejaview-tools --test provisioning -- --ignored

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
