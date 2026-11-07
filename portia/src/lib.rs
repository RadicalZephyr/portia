//! Lightweight Functional Reactive Programming, spun from Rust.
//!
//! Portia is a library for composing your own Rust data structures and
//! functions into a program that reacts to events. It goes on top of the
//! code you already have: your types stay your types and your functions
//! stay plain functions. You tell Portia which events feed which of your
//! functions, and its engine works out a valid order to compute every
//! update in.
//!
//! Portia has a model of time. Things happen in transactions, and each
//! transaction is a single instant: everything an event causes happens in
//! that instant, together, so nothing ever sees a half-updated state. A
//! stream's events happen at an instant, and a cell has a value at every
//! instant, which events change from one instant to the next. Each piece's
//! meaning is fixed by those rules, so pieces you build and test separately
//! compose predictably.
//!
//! You describe how events flow once, in a build closure. Then your I/O
//! code drives the graph you built: it sends events in and listens to what
//! comes out. The graph does no I/O itself, so the same graph runs against
//! stdin or against a test. The example below is the second kind.
//!
//! If you know FRP: Portia implements Sodium's denotational semantics, with
//! an API that separates building FRP logic from driving it with I/O. The
//! design is recorded in the RFDs at <https://github.com/portia-rs/rfd>.
//!
//! # The pieces
//!
//! - [`Runtime`] holds the graph. It's where events enter your program and
//!   results leave it, and dropping it shuts everything down.
//! - The build closure you pass to [`Runtime::build`] is where you add to
//!   the graph. Its argument, `b`, is a [`BuildContext`], and a method that
//!   takes `b` adds a node. A part of a graph is an ordinary function that
//!   takes `b` and returns tokens, so you combine Portia programs the way
//!   you combine any other functions.
//! - A [`Stream`] is events that happen at an instant: each line read, each
//!   click. A stream has one consumer. [`share`](Source::share) it to get a
//!   [`Shared`], which takes any number of consumers and clones each event
//!   for each one.
//! - A [`Cell`] has a value at every instant, such as the word count so
//!   far. A [`State`] is a cell updated in place, and you read it the same
//!   way.
//! - An [`Input`] is the I/O side of a stream: what you
//!   [`send`](Runtime::send) to. Each send is a transaction of its own, and
//!   [`Runtime::transaction`] puts several sends in one.
//! - A [`Listener`] runs your code with a stream's events or a cell's
//!   values, after each transaction. Dropping it stops it.
//!
//! Streams, cells and inputs are small handles, called tokens, that name
//! parts of the graph. The data itself lives in the runtime, which collects
//! the parts nothing uses any more.
//!
//! # Example
//!
//! Read lines from stdin and keep a running word count and the longest line
//! so far, printing both after every line. In a clone of the repository,
//! `cargo run --example word_count_mut` runs it.
//!
//! ```rust
//! use portia::prelude::*;
//!
//! fn main() -> std::io::Result<()> {
//!     let mut rt = Runtime::local();
//!     let (lines_in, summary) = rt.build(|b| {
//!         let (lines, lines_in) = b.input::<String>();
//!         let lines = lines.share(b); // two consumers below
//!         let words = lines.accumulate_mut(b, 0_usize, |line, n: &mut usize| {
//!             *n += line.split_whitespace().count();
//!         });
//!         let longest = lines.accumulate_mut(b, String::new(), |line, best: &mut String| {
//!             if line.chars().count() > best.chars().count() {
//!                 *best = line;
//!             }
//!         });
//!         let summary = (words, longest).lift(b, |n, l| format!("{n} words, longest: {l:?}"));
//!         (lines_in, summary)
//!     });
//!
//!     let mut i = 0;
//!     let expected = [
//!         "0 words, longest: \"\"",
//!         "2 words, longest: \"Hello, World!\"",
//!         "3 words, longest: \"Hello, World!\"",
//!         "6 words, longest: \"Try Portia Today!\"",
//!     ];
//!     // Fires once now, with the initial values, then again on every step.
//!     let _listener = rt.listen_cell(summary, move |s| {
//!         assert_eq!(s, expected[i]);
//!         i += 1;
//!     });
//!     for line in ["Hello, World!", "Foo", "Try Portia Today!"] {
//!         rt.send(lines_in, line.to_string());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Every value a cell holds implements [`Trace`], so the collector
//! can find any tokens inside it. The standard library's types
//! already do. For your own state types, turn on the `derive`
//! feature, `portia = { version = "0.0.1", features = ["derive"] }`,
//! and add `#[derive(Trace)]`. Wrap a type from another crate in
//! [`Leaf`] if it never contains tokens, or your own newtype if it
//! can contain tokens and you need to provide a [`Trace`] impl.
//!
//! # Rules for code Portia runs
//!
//! Node code, the closures you pass to the graph, and listeners have to
//! keep to these rules. [`rules`] explains each one.
//!
//! - **Don't panic.** Not in node code, a listener or the `Drop` of a
//!   value the graph holds. A panic poisons the runtime for good: if
//!   that happens you must drop it and build a new one. Return errors
//!   as values, in a `Result` or an `Option`, and watch for `unwrap`,
//!   indexing and integer overflow.
//! - **Be a function of your inputs.** Mutation inside one call is
//!   fine.  State carried between calls in a captured `RefCell`
//!   isn't, put it in a Portia [`Cell`] instead.
//! - **No effects you rely on.** The engine decides when your closure
//!   runs and how often.
//! - **Declare every token a closure captures with
//!   [`depends`](Builder::depends).** Otherwise its node can be
//!   collected, and the closure's next use of it panics, in release builds
//!   too.
//!
//! # Status
//!
//! This release works and is usable. Nothing is `todo!()`, the examples in
//! the documentation run, and the guarantees the RFDs make are fixed by
//! `compile_fail` doc tests.
//!
//! Version 0.0.1 is a proof-of-concept spike written by Claude, and
//! I'm rewriting the engine underneath by hand. The tests pin its
//! behavior, hand-made mutations of the engine show the tests would
//! catch a change in it, and differential tests against Sodium's
//! denotational semantics, run by GHC as an oracle, show it's the
//! right behavior.
//!
//! 0.0.1 is a frozen snapshot with no patch releases planned. The next
//! release is 0.1.0, written by hand, and it aims to keep this API close.
//! Bug reports and API feedback go in
//! [GitHub issues](https://github.com/portia-rs/portia/issues).
//!
//! # What's in the box
//!
//! - **Inputs:** inputs, coalescing or not, input cells, constants and
//!   `never`.
//! - **Adapters**, which fuse into the one node that materializes them:
//!   `map`, `filter`, `filter_map`, `map_to`, `snapshot`, `gate` and `once`.
//! - **Materializers:** `hold`, `node`, `share`, `merge` and `or_else`.
//! - **Cells:** the accumulators `accumulate`, `accumulate_mut` and `scan`;
//!   read-through cells with `map_cell` and `lift`; the stream views `steps`
//!   and `steps_with_current`; and [`State`] and [`CellRef`].
//! - **Loops:** `cell_loop` with [`CellLoop`], `state_loop` with
//!   [`StateLoop`] and `stream_loop` with [`StreamLoop`], with the
//!   dependency graph kept acyclic.
//! - **Child transactions:** `split` emits the elements of an event in the
//!   children of its transaction, and `defer` the event in its first child.
//! - **Switches:** `switch_cell`, over cells and over states, and
//!   `switch_stream`.
//! - **[`construct`](Source::construct)**, which runs a closure with a
//!   builder at each event of a stream, in the middle of its transaction.
//! - **Collection** (RFD 3): a node lives while a root reaches it, a live
//!   [`Listener`] or [`Anchor`] or a registration waiting in a handle's
//!   queue, with [`Runtime::build_anchored`] and [`Anchored`] for roots
//!   that end, and [`Trace`] for tokens held in a stateful cell's value.
//! - **The I/O boundary** (RFD 6, RFD 7): [`InputSlot`],
//!   [`pump`](Runtime::pump), the [`Io`] and [`RemoteIo`] handles, and
//!   [`set_waker`](Runtime::set_waker).
//!
//! # Targets
//!
//! The core is `no_std` over `alloc`. The `std` feature, on by default, adds
//! the thread-id guard on [`RemoteIo`], `Trace` for the standard collections
//! and `Instant`, and the standard mutex under input slots. Where the target
//! has no pointer atomics, on a Cortex-M0, `Threaded`, `RemoteIo` and the
//! unit queue do not exist, and the path from an interrupt handler into the
//! graph is an [`InputSlot`]. The `critical-section` feature guards slots on bare metal;
//! a web build keeps `std` (RFD 7). A slot and a remote's inbox need one of
//! the two locks: with no `unsafe` in the crate there is none to build from
//! atomics, so a `no_std` build without `critical-section` has neither
//! slots nor `RemoteIo`, and keeps [`pump`](Runtime::pump),
//! [`set_waker`](Runtime::set_waker) and the [`Io`], whose queue is the
//! runtime's own and needs no lock.
//!
//! # Transactions
//!
//! Every `send` from outside is a transaction of its own, and
//! [`Runtime::transaction`] puts several sends in one instant. RFD 2's
//! click counter shows both:
//!
//! ```
//! use std::cell::RefCell;
//! use std::rc::Rc;
//!
//! use portia::prelude::*;
//!
//! struct Click;
//!
//! let mut rt = Runtime::local();
//! let (clicks_in, label) = rt.build(|b| {
//!     let (clicks, clicks_in) = b.input::<Click>();
//!     let count = clicks.accumulate(b, 0u32, |_, n| n + 1);
//!     let label = count.map_cell(b, |n| n.to_string());
//!     (clicks_in, label) // whatever build returns is rooted for the runtime's life
//! });
//!
//! let shown = Rc::new(RefCell::new(Vec::new()));
//! let _listener = rt.listen_cell(label, {
//!     let shown = shown.clone();
//!     move |text| shown.borrow_mut().push(text.clone()) // fires now, then on every step
//! });
//! rt.send(clicks_in, Click); // one transaction
//! rt.transaction(|tx| {
//!     tx.send(clicks_in, Click); // several sends, one instant
//! });
//! assert_eq!(*shown.borrow(), ["0", "1", "2"]);
//! assert_eq!(rt.sample(label), "2");
//! ```

#![warn(missing_docs)]
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod build;
mod builder;
mod capabilities;
mod cell;
mod engine;
mod error;
mod guard;
mod io;
mod lift;
mod mode;
pub mod prelude;
#[cfg(doctest)]
mod refusals;
pub mod rules;
mod runtime;
#[cfg(any(feature = "std", feature = "critical-section"))]
mod slot;
#[cfg(feature = "smoke")]
mod smoke;
mod source;
mod token;
mod trace;

pub use build::{BuildContext, CellLoop, StateLoop, StreamLoop};
pub use builder::{Builder, Portable, Stores};
pub use cell::CellRef;
#[cfg(feature = "statistics")]
pub use engine::Statistics;
pub use error::{
    IoError, IoTransactionError, PoisonedError, PumpError, SendError, TokenError,
    TransactionListenError, TransactionSendError,
};
pub use io::Io;
pub use lift::Lift;
#[cfg(target_has_atomic = "ptr")]
pub use mode::Threaded;
pub use mode::{Accepts, Local, Mode};
/// Derives [`Trace`](trait@Trace) for a struct or an enum, with
/// `#[trace(skip)]` for a field that cannot hold tokens.
#[cfg(feature = "derive")]
pub use portia_derive::Trace;
pub use runtime::{
    Anchor, Anchored, CollectionPolicy, IoTransaction, Listener, Runtime, Transaction,
};
#[cfg(all(
    target_has_atomic = "ptr",
    any(feature = "std", feature = "critical-section")
))]
pub use runtime::{RemoteIo, RemoteTransaction};
#[cfg(any(feature = "std", feature = "critical-section"))]
pub use slot::InputSlot;
#[cfg(feature = "smoke")]
#[doc(hidden)]
pub use smoke::smoke;
pub use source::{Filter, FilterMap, Gate, Map, MapTo, Node, Once, Snapshot, Source};
pub use token::{Cell, Input, Shared, State, Stream, TokenRef};
pub use trace::{Leaf, Trace, Tracer};
