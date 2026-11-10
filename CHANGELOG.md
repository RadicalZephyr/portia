# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2026-11-07

The first release: a proof-of-concept spike written by Claude, to show that the interface
design is buildable and can be held to Sodium's denotational semantics. It works, and
nothing is `todo!()`. It's a frozen snapshot with no patch releases planned. 0.1.0 is a
rewrite of the engine by hand, and it aims to keep this API close.

### Added

- `portia`, an FRP library implementing Sodium's denotational semantics, and
  `portia-derive`, which provides `#[derive(Trace)]`.
- A `no_std` core over `alloc`, checked for `thumbv6m`, `thumbv7m`, `thumbv7em` and
  `wasm32`.
- `Runtime`, which separates building the graph from driving it: build closures add to
  the graph through a `BuildContext`, and I/O code drives it with `send`, `transaction`
  and the `listen` family.
- The I/O boundary: `InputSlot`, `pump`, the `Io` and `RemoteIo` handles, and
  `set_waker`.
- Single-threaded and multi-threaded runtimes, `Runtime::local()` and
  `Runtime::threaded()`, and `Portable` builders for graph code that runs under either.
- Tokens, the small handles that name parts of the graph. The data itself lives in the
  runtime.
  - `Stream` is a stream of events that each happen at an instant, and has at most one
    consumer. `share` turns it into a `Shared`, which clones each event for any number of
    consumers. Both implement `Source`, the trait that holds the stream operations below.
  - `Cell` has a value at every instant, which can change in response to events. A `State`
    is a cell updated in place. You read it the same way, through `CellRef`, but it has no
    `steps` or `calm`.
  - `Input` is the I/O side of a stream or a cell: what `send` sends to.
- Inputs, coalescing or not, input cells, constants and `never`.
- Adapters that fuse into the one node that materializes them: `map`, `filter`,
  `filter_map`, `map_to`, `snapshot`, `gate` and `once`.
- Materializers: `hold`, `node`, `share`, `merge` and `or_else`.
- Cells: the accumulators `accumulate`, `accumulate_mut` and `scan`; `map_cell` and
  `lift`; the stream views `steps` and `steps_with_current`; and `calm`.
- Switches: `switch_cell`, over cells and over states, and `switch_stream`.
- `construct`, which runs a closure with a builder at each event of a stream, in the
  middle of its transaction.
- Loops: `cell_loop`, `state_loop` and `stream_loop`, with the dependency graph kept
  acyclic.
- Child transactions: `split` and `defer`.
- Garbage collection of nodes nothing reaches: `Listener` and `Anchor` as roots,
  `build_anchored` and `Anchored` for roots that end, `Trace` for tokens held in a
  stateful cell's value, and `CollectionPolicy`.
- The `rules` module, which lists the rules for code Portia runs, and `compile_fail` doc
  tests that pin the guarantees the RFDs make.
- The features `std` and `derive`, on by default, `critical-section` for input slots on
  bare metal, and `statistics` for per-transaction phase counters.
- Minimum supported Rust version 1.85, edition 2024, licensed Apache-2.0.

[unreleased]: https://github.com/portia-rs/portia/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/portia-rs/portia/releases/tag/v0.0.1
