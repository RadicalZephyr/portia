//! Wall-clock benchmarks, run by hand with `cargo bench -p portia-bench --bench shapes`.
//!
//! The bar is a factor of three against the baseline with a realistic
//! payload; the trivial-payload numbers are information.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use portia_bench::{
    FanOut, FanOutBaseline, Frame, FrameBaseline, Shallow, ShallowBaseline, payload,
};

fn the_payload(c: &mut Criterion) {
    c.bench_function("payload", |b| b.iter(|| payload(black_box(7))));
}

fn shallow(c: &mut Criterion) {
    let mut group = c.benchmark_group("shallow");
    for (name, share, heavy) in [
        ("plain", false, false),
        ("share", true, false),
        ("plain_payload", false, true),
        ("share_payload", true, true),
    ] {
        group.bench_function(BenchmarkId::new("portia", name), |b| {
            let mut shape = Shallow::new(share, heavy);
            let mut k = 0u64;
            b.iter(|| {
                k += 1;
                shape.send(black_box(k));
            });
            black_box(shape.value());
        });
    }
    for (name, heavy) in [("trivial", false), ("payload", true)] {
        group.bench_function(BenchmarkId::new("baseline", name), |b| {
            let mut base = ShallowBaseline { held: 0, heavy };
            let mut k = 0u64;
            b.iter(|| {
                k += 1;
                base.send(black_box(k));
                black_box(base.held)
            });
        });
    }
    group.finish();
}

fn frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame");
    group.bench_function("portia", |b| {
        let mut shape = Frame::new();
        let mut k = 0u64;
        b.iter(|| {
            k += 1;
            shape.frame(black_box(k));
        });
        black_box(shape.checksum());
    });
    // Information, not held to the bar: the baseline has no transactions.
    for (name, transaction) in [("unbatched_send", false), ("unbatched_transaction", true)] {
        group.bench_function(BenchmarkId::new("portia", name), |b| {
            let mut shape = Frame::new();
            let mut k = 0u64;
            b.iter(|| {
                k += 1;
                if transaction {
                    shape.unbatched_transaction(black_box(k));
                } else {
                    shape.unbatched_send(black_box(k));
                }
            });
            black_box(shape.checksum());
        });
    }
    group.bench_function("baseline", |b| {
        let mut base = FrameBaseline::new();
        let mut k = 0u64;
        b.iter(|| {
            k += 1;
            base.frame(black_box(k));
        });
        black_box(base.checksum());
    });
    group.finish();
}

/// Islands in the graph [`frame_batch`] sends into, and the batch sizes it
/// sends. The graph stays the same size so only the batch size changes.
const BATCH_GRAPH: usize = 10_000;
const BATCH_SIZES: [usize; 7] = [1, 10, 100, 300, 1000, 3000, 10_000];

/// Information, not held to the bar: the frame shape's cost per input
/// against how many inputs send in one transaction, next to the same sends
/// unbatched.
fn frame_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_batch");
    let mut shape = Frame::with_inputs(BATCH_GRAPH);
    let mut k = 0u64;
    for n in BATCH_SIZES {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("batched", n), &n, |b, &n| {
            b.iter(|| {
                k += 1;
                shape.batch(n, black_box(k));
            });
        });
        group.bench_with_input(BenchmarkId::new("unbatched", n), &n, |b, &n| {
            b.iter(|| {
                k += 1;
                shape.unbatched_batch(n, black_box(k));
            });
        });
    }
    black_box(shape.checksum());
    group.finish();
}

fn fan_out(c: &mut Criterion) {
    let mut group = c.benchmark_group("fan_out");
    group.bench_function("portia", |b| {
        let mut shape = FanOut::new();
        let mut k = 0u64;
        b.iter(|| {
            k += 1;
            shape.send(black_box(k));
        });
        black_box(shape.sum());
    });
    group.bench_function("baseline", |b| {
        let mut base = FanOutBaseline::new();
        let mut k = 0u64;
        b.iter(|| {
            k += 1;
            base.send(black_box(k));
        });
        black_box(base.sum());
    });
    group.finish();
}

criterion_group!(benches, the_payload, shallow, frame, frame_batch, fan_out);
criterion_main!(benches);
