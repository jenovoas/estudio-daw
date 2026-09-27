# Proposal

## Why

`RenderPlanBuilder::connect` suggests a routed graph, but audio nodes only receive one shared mutable block in sequence. Branching sources therefore overwrite one another and the API describes behavior the processor cannot provide.

## What Changes

- **BREAKING** Remove `connect` and DAG/cycle compilation from the current processor API.
- Define `RenderPlan` as an insertion-ordered, in-place effects chain.
- Keep parallel source summing explicit in mixer nodes that own preallocated scratch buffers.
- Update the CLI pipeline and architecture documentation; add a regression proving sequential chain semantics.

## Capabilities

### New Capabilities

- `audio-render-chain`: explicit processing order and parallel mixing contract for the current RenderPlan engine.

### Modified Capabilities

None. `openspec/specs/` has no synchronized main specs, so this establishes a canonical capability contract.

## Impact

Affected code is `crates/audio-engine`, its CLI plan construction, and `docs/audio-render-plan.md`. No callback allocation or serialized project behavior changes.
