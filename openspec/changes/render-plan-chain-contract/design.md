# Design

## Plan model

`RenderPlanBuilder` stores nodes in insertion order. `build()` transfers that ordered list into an immutable-topology plan; `process()` calls each node in order on the same interleaved buffer. A source node replaces/fills the block and subsequent insert/master nodes transform it in place. This is a chain, not a DAG.

## Parallel sources

Parallel instruments must be wrapped in an explicit mixer such as `InstrumentMixerNode`, which renders each source into preallocated scratch, sums samples, and writes the mixed block. A future graph implementation requires a separate audio-buffer input/output node contract and precompiled buffer allocation plan.

## API impact

Remove `GraphError`, per-node dependency lists, topological sort, and `connect`. `build()` becomes infallible. Update in-repository call sites to express their intended chain solely through insertion order.

## Verification

Test that two additive test nodes run in insertion order on the same block; retain mixer tests to prove parallel summing is explicit and allocation-free. Run formatting, workspace tests/check, strict OpenSpec validation, and diff checks.
