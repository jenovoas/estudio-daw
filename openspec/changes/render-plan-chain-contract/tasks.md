# Tasks

## 1. Align API and implementation

- [x] 1.1 Remove DAG connection/dependency storage and expose an infallible insertion-ordered chain builder.
- [x] 1.2 Update CLI plan construction to use insertion order and keep source summing in explicit mixer nodes.
- [x] 1.3 Update render-plan documentation to remove graph/DAG claims and describe chain/mixer contracts.
- [x] 1.4 Add regression coverage for deterministic chain ordering and explicit source mixing.

## 2. Verify and record

- [x] 2.1 Run `cargo fmt --all`, `cargo test --workspace -- --test-threads=1`, `cargo check --workspace`, strict OpenSpec validation, and `git diff --check`.
- [ ] 2.2 Append the verified SHA and outcome to AGENTS/vault handoff.
