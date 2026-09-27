# audio-render-chain Specification

## Purpose

Defines deterministic in-place DSP chain behavior and the explicit contract for mixing parallel instrument sources.

## Requirements

### Requirement: Render plans have explicit in-place chain semantics
The current RenderPlan MUST process nodes in insertion order on one shared in-place audio block. The builder MUST NOT expose a connection API that implies independent graph branches or automatic summing.

#### Scenario: Ordered processing chain
- **WHEN** multiple nodes are added and a block is processed
- **THEN** each node receives the block after the preceding node has processed it
- **AND** processing order is deterministic and matches insertion order

### Requirement: Parallel instrument sources are explicitly mixed
Parallel sources MUST be combined by a mixer that provides separate preallocated scratch to each source and sums their samples before writing the output block.

#### Scenario: Two simultaneous instrument sources
- **WHEN** a mixer renders two source nodes for one block
- **THEN** output contains the sum of both source signals
- **AND** the callback performs no allocation to mix them
