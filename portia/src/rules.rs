//! The rules for code Portia runs, and why each one is there.
//!
//! Portia runs your code in two places: node code, the closures you pass
//! to `map`, `accumulate`, `lift`, `construct` and the rest, and listeners.
//! The graph only means what Sodium's semantics say it means if that code
//! keeps to these rules. The compiler checks some of what this needs, but
//! not all of it, and these are the parts it can't check.
//!
//! # Don't panic
//!
//! Nothing Portia calls may panic: not node code, not a listener, and not
//! the `Drop` of a value the graph holds. A panic leaves a transaction half
//! done, so the runtime is poisoned. Every call after that panics, or
//! returns `Poisoned` from its `try_` variant, and nothing clears the
//! poison. The only way out is to drop the runtime and build a new one.
//! That applies even where panics abort instead of unwinding, so a panic
//! in node code is never local to the closure it happened in.
//!
//! Return errors as values instead. A stream of `Result`s carries the error
//! to whatever handles it, and `filter_map` drops what you don't want:
//!
//! ```rust
//! use portia::prelude::*;
//!
//! let mut rt = Runtime::local();
//! let (lines_in, last) = rt.build(|b| {
//!     let (lines, lines_in) = b.input::<String>();
//!     let parsed = lines.map(|line: String| line.trim().parse::<u32>().map_err(|e| e.to_string()));
//!     let last = parsed.hold(b, Ok(0));
//!     (lines_in, last)
//! });
//! rt.send(lines_in, "twelve".to_string());
//! assert!(rt.sample(last).is_err()); // an error, and the runtime still works
//! rt.send(lines_in, "12".to_string());
//! assert_eq!(*rt.sample(last), Ok(12));
//! ```
//!
//! Watch for the panics Rust hides: `unwrap`, `expect`, indexing, and
//! integer overflow. Overflow is the worst of them, because it panics in a
//! debug build and wraps in a release build, so the same graph can poison
//! in one profile and not the other. Use `checked_`, `saturating_` or
//! `wrapping_` arithmetic to say which you mean.
//!
//! Here's what poisoning looks like:
//!
//! ```rust
//! use std::panic::{AssertUnwindSafe, catch_unwind};
//!
//! use portia::prelude::*;
//! use portia::SendError;
//!
//! let mut rt = Runtime::local();
//! let (numbers_in, _total) = rt.build(|b| {
//!     let (numbers, numbers_in) = b.input::<u8>();
//!     let total = numbers.accumulate(b, 0u8, |n, total| total.checked_add(n).unwrap());
//!     (numbers_in, total)
//! });
//! rt.send(numbers_in, 200);
//! let overflowed = catch_unwind(AssertUnwindSafe(|| rt.send(numbers_in, 100)));
//! assert!(overflowed.is_err());
//! assert_eq!(rt.try_send(numbers_in, 1), Err(SendError::Poisoned)); // for good
//! ```
//!
//! # Be a function of your inputs
//!
//! Node code's output depends on its inputs and nothing else. Mutation that
//! stays inside one call is fine: build a scratch `Vec`, sort it, return
//! it. State carried from one call to the next isn't. Most node closures
//! are `Fn`, which rules out `FnMut`, but a captured `RefCell`, `Cell` or
//! static gets around that, so it's up to you. State that lasts between
//! events belongs in the graph, in `accumulate`, `scan` or a loop, where
//! the engine knows about it.
//!
//! # No effects you rely on
//!
//! The engine makes no guarantees about when it calls your closure,
//! or how many times. A `map_cell` closure runs when something reads
//! the cell, not when its input changes: with nothing reading it, a
//! send doesn't run it at all. So node code doesn't write files, send
//! messages or count its own calls. A `println!` while you debug is
//! fine, as long as you don't read anything into how many lines it
//! prints, or when. Effects go in listeners, which are I/O code.
//!
//! # Declare what a closure captures with `depends`
//!
//! A node lives while a root reaches it (RFD 3): what `build` returned, a
//! live [`Listener`](crate::Listener) or a live [`Anchor`](crate::Anchor).
//! The collector follows what a node depends on and the tokens in a cell's
//! value, but it can't look inside a closure. So a token a `move` closure
//! captures has to be declared with [`Builder::depends`](crate::Builder::depends),
//! or the node it names can be collected while the closure still uses it.
//!
//! The closure's next use of that token panics, in a release build as well
//! as a debug one, with the place the node was created in the message. Like
//! any panic in node code, that poisons the runtime. When it happens
//! depends on when a collection runs. A node that nothing reaches when
//! `build` returns is collected before it returns, so you find out on the
//! first event. A node that only becomes unreachable later fails at some
//! later collection, which can make it look intermittent. The
//! `depends` documentation shows the declaration.
