//! `calm`, Sodium's `calm`: a cell with the same value at every instant,
//! which steps only when the value changes. Each step is compared with the
//! value from before the instant, so a `lift` that steps without changing
//! its value, or a hold of an equal event, doesn't step the calmed cell.

use std::cell::RefCell;
use std::rc::Rc;

use proptest::prelude::*;

use portia::prelude::*;

/// A shared log and a closure that appends to it.
fn recorder<T: 'static>() -> (Rc<RefCell<Vec<T>>>, impl FnMut(T) + 'static) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let writer = log.clone();
    (log, move |v| writer.borrow_mut().push(v))
}

/// Two input cells, a `lift` of their sum's parity, which steps whenever
/// either input does, and the calmed parity.
fn parity_graph() -> (
    Runtime,
    Input<u32>,
    Input<u32>,
    portia::Cell<u32>,
    portia::Cell<u32>,
) {
    let mut graph = Runtime::local();
    let (a_in, c_in, parity, calm) = graph.build(|b| {
        let (a, a_in) = b.input_cell(1u32);
        let (c, c_in) = b.input_cell(10u32);
        let parity = (a, c).lift(b, |a, c| (a + c) % 2);
        let calm = parity.calm(b);
        (a_in, c_in, parity, calm)
    });
    (graph, a_in, c_in, parity, calm)
}

#[test]
fn calm_steps_only_when_the_value_changes() {
    let (mut graph, a_in, c_in, parity, calm) = parity_graph();
    let (raw, mut on_raw) = recorder();
    graph.listen_steps(parity, move |v: &u32| on_raw(*v)).keep();
    let (seen, mut on) = recorder();
    graph.listen_steps(calm, move |v: &u32| on(*v)).keep();
    graph.send(a_in, 3); // 13: odd, as before
    graph.send(a_in, 4); // 14: even
    graph.send(c_in, 12); // 16: even
    graph.transaction(|tx| {
        tx.send(a_in, 5);
        tx.send(c_in, 13); // 18: even, with both inputs stepping
    });
    graph.send(c_in, 14); // 19: odd
    assert_eq!(*raw.borrow(), [1, 0, 0, 0, 1]);
    assert_eq!(*seen.borrow(), [0, 1]);
}

#[test]
fn calm_has_the_same_value_as_its_cell_at_every_instant() {
    let (mut graph, a_in, c_in, parity, calm) = parity_graph();
    assert_eq!(graph.sample(calm), graph.sample(parity));
    for (a, c) in [(3, 10), (4, 10), (4, 12), (5, 13), (5, 14)] {
        graph.transaction(|tx| {
            tx.send(a_in, a);
            tx.send(c_in, c);
        });
        assert_eq!(graph.sample(calm), graph.sample(parity));
    }
}

#[test]
fn a_cell_listener_on_calm_fires_now_and_then_on_each_change() {
    let mut graph = Runtime::local();
    let (level_in, calm) = graph.build(|b| {
        let (level, level_in) = b.input_cell(5u32);
        (level_in, level.calm(b))
    });
    let (seen, mut on) = recorder();
    graph.listen_cell(calm, move |v: &u32| on(*v)).keep();
    for n in [5, 6, 6, 5] {
        graph.send(level_in, n);
    }
    assert_eq!(*seen.borrow(), [5, 6, 5]);
}

/// Built by a `construct` in an instant where the cell steps: the calmed
/// cell starts at the value from before the instant, so the step comes
/// through if it changes the value and not if it doesn't.
#[test]
fn calm_built_in_an_instant_where_its_cell_steps_starts_from_before_the_instant() {
    let mut graph = Runtime::local();
    let (go_in, made) = graph.build(|b| {
        let (go, go_in) = b.input::<u32>();
        let go = go.share(b);
        let held = go.hold(b, 5u32);
        let made = go.construct(b, move |b, _| {
            let calm = held.calm(b);
            let log = calm
                .steps(b)
                .accumulate(b, Vec::new(), |v, log: &Vec<u32>| {
                    let mut log = log.clone();
                    log.push(v);
                    log
                });
            b.anchor((calm, log))
        });
        b.depends(&made, &[&held]);
        (go_in, made)
    });
    let (rows, on) = recorder();
    graph.listen(made, on).keep();
    graph.send(go_in, 5); // equal to the value before: no step
    graph.send(go_in, 6); // a change: steps the second row in its creation instant
    let rows = rows.borrow();
    let (first, first_log) = *rows[0];
    let (second, second_log) = *rows[1];
    // The second send stepped the first row too.
    assert_eq!(*graph.sample(first_log), [6]);
    assert_eq!(*graph.sample(second_log), [6]);
    assert_eq!(*graph.sample(first), 6);
    assert_eq!(*graph.sample(second), 6);
}

/// `calm` reads the cell's value when it's built, as `sample` does, so it
/// can't calm the forward token of a loop that isn't closed yet.
#[test]
#[should_panic(expected = "a cell loop sampled before it is closed")]
fn calm_of_an_open_loop_forward_panics() {
    Runtime::local().build(|b| {
        let (forward, cell_loop) = b.cell_loop::<u32>();
        let _calm = forward.calm(b);
        let (numbers, _numbers_in) = b.input::<u32>();
        let held = numbers.hold(b, 0);
        cell_loop.close(b, held);
    });
}

/// A unit the property drives: a send to one input, or one transaction
/// that sends to both.
#[derive(Clone, Debug)]
enum Unit {
    A(u32),
    C(u32),
    Both(u32, u32),
}

fn units() -> impl Strategy<Value = Vec<Unit>> {
    // Small values, so equal results are common.
    let value = 0u32..4;
    prop::collection::vec(
        prop_oneof![
            value.clone().prop_map(Unit::A),
            value.clone().prop_map(Unit::C),
            (value.clone(), value).prop_map(|(a, c)| Unit::Both(a, c)),
        ],
        0..32,
    )
}

proptest! {
    /// After every unit the calmed cell holds the cell's value, and it has
    /// stepped exactly when that value differs from the one before the unit.
    #[test]
    fn calm_steps_exactly_at_value_changes(units in units()) {
        let mut graph = Runtime::local();
        let (a_in, c_in, cell, calm) = graph.build(|b| {
            let (a, a_in) = b.input_cell(0u32);
            let (c, c_in) = b.input_cell(0u32);
            let cell = (a, c).lift(b, |a, c| (a + c) % 3);
            (a_in, c_in, cell, cell.calm(b))
        });
        let (seen, mut on) = recorder();
        graph.listen_steps(calm, move |v: &u32| on(*v)).keep();
        let (mut a, mut c) = (0u32, 0u32);
        let mut expected = Vec::new();
        for unit in units {
            let before = (a + c) % 3;
            match unit {
                Unit::A(n) => {
                    a = n;
                    graph.send(a_in, n);
                }
                Unit::C(n) => {
                    c = n;
                    graph.send(c_in, n);
                }
                Unit::Both(m, n) => {
                    (a, c) = (m, n);
                    graph.transaction(|tx| {
                        tx.send(a_in, m);
                        tx.send(c_in, n);
                    });
                }
            }
            let after = (a + c) % 3;
            if after != before {
                expected.push(after);
            }
            prop_assert_eq!(graph.sample(calm), graph.sample(cell));
        }
        prop_assert_eq!(&*seen.borrow(), &expected);
    }
}
