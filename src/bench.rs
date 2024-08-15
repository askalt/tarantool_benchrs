use std::time::{Duration, Instant};

/// Measure time for the specific function.
pub fn run_bench<State>(
    retries: usize,
    f: impl Fn(&mut State),
    setup: impl Fn(&mut State),
    teardown: impl Fn(&mut State),
    mut state: State,
) -> Vec<Duration> {
    let mut res = vec![];
    for _ in 0..retries {
        setup(&mut state);
        let before = Instant::now();
        f(&mut state);
        let after = Instant::now();
        res.push(after - before);
        teardown(&mut state);
    }
    res
}
