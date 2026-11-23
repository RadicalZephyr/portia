//! Instruction-count benchmarks, run in CI as the regression gate.
//!
//! CI runs this twice in one job, first on the base commit and then on the
//! head commit, so the second run is compared against the first and fails on a
//! regression beyond the configured limit.

use gungraun::{
    Callgrind, EventKind, LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main,
};
use std::hint::black_box;

use portia_bench::{FanOut, Frame, Shallow};

// A thousand transactions of the shallow shape, build included.
#[library_benchmark]
#[bench::plain(false)]
#[bench::share(true)]
fn shallow(share: bool) -> u64 {
    let mut shape = Shallow::new(share, false);
    for k in 0..1000 {
        shape.send(black_box(k));
    }
    black_box(shape.value())
}

// Ten frames of the frame shape, build included.
#[library_benchmark]
fn frame() -> u64 {
    let mut shape = Frame::new();
    for k in 0..10 {
        shape.frame(black_box(k));
    }
    black_box(shape.checksum())
}

// Ten frames' worth of sends to the frame shape, one transaction per send,
// build included: through `send`, or through `transaction` with one send.
#[library_benchmark]
#[bench::send(false)]
#[bench::transaction(true)]
fn frame_unbatched(transaction: bool) -> u64 {
    let mut shape = Frame::new();
    for k in 0..10 {
        if transaction {
            shape.unbatched_transaction(black_box(k));
        } else {
            shape.unbatched_send(black_box(k));
        }
    }
    black_box(shape.checksum())
}

// A thousand transactions of the fan-out shape, build included.
#[library_benchmark]
fn fan_out() -> u64 {
    let mut shape = FanOut::new();
    for k in 0..1000 {
        shape.send(black_box(k));
    }
    black_box(shape.sum())
}

library_benchmark_group!(name = shapes; benchmarks = shallow, frame, frame_unbatched, fan_out);

main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::default().soft_limits([(EventKind::Ir, 5.0)]).fail_fast(true));
    library_benchmark_groups = shapes
);
