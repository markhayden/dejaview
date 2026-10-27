# Review checklist

Applied to every milestone branch before the owner's checkpoint review, by the review and test
skills (`/agent-skills:review`, `/agent-skills:test`) where available, otherwise by hand using this
list.

## Code review, five axes

- **Correctness.** Every acceptance criterion of every task in the milestone has a test that
  fails without the change. Error paths reach the documented `error:` class. No path lets an
  operational failure become a verdict.
- **Readability.** Plain structs and enums; names match the spec's vocabulary (obligation,
  instance, checklist item, envelope, frame, decision, run); no `unwrap` outside tests.
- **Architecture.** Crate edges unchanged or changed with the graph file; boundary types derive
  `Serialize`, `Deserialize`, `JsonSchema`, use `deny_unknown_fields` and carry a version; the
  core has no I/O it was not given.
- **Security.** No credential in any output path; image and frame ids resolve only through the
  registry; untrusted text stays in data blocks; pack environments filtered; no network in tests
  without `DEJAVIEW_LIVE=1`.
- **Performance.** `cargo test --workspace` under five minutes offline; no image decoded twice;
  bounded decode sizes.

## Test review

- Every state-machine transition and aggregation row in the spec has a test.
- Fixtures are replayed, never fetched, in `cargo test`.
- Negative tests exist for every validation rule introduced.
- No test was skipped or disabled to make the suite pass.

## Documentation

- `SPEC.md`, `PLAN.md`, `tasks/todo.md`, `README.md` and `docs/` match the code in the same commit.
