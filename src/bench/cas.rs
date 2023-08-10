use core_affinity::CoreId;
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, Ordering};
use quanta::Clock;
use super::Count;

const PING: bool = false;
const PONG: bool = true;

#[derive(Default)]
pub struct Bench;

impl super::Bench for Bench {
    // The two threads modify the same cacheline.
    // This is useful to benchmark spinlock performance.
    fn run(
        &self,
        (ping_core, pong_core): (CoreId, CoreId),
        clock: &Clock,
        num_round_trips: Count,
        num_samples: Count,
    ) -> Vec<f64> {
        // Mind first-touch binds memory to current numa node
        core_affinity::set_for_current(pong_core);

        // Shared states
        let ref barrier = Barrier::new(2);
        let ref flag = *Box::new(AtomicBool::new(PING));

        crossbeam_utils::thread::scope(|s| {
            let pong = s.spawn(move |_| {
                core_affinity::set_for_current(pong_core);

                barrier.wait();
                for _ in 0..(num_round_trips*num_samples) {
                    while flag.compare_exchange(PING, PONG, Ordering::Relaxed, Ordering::Relaxed).is_err() {}
                }
            });

            let ping = s.spawn(move |_| {
                core_affinity::set_for_current(ping_core);

                let mut results = Vec::with_capacity(num_samples as usize);

                barrier.wait();

                for _ in 0..num_samples {
                    let start = clock.raw();
                    for _ in 0..num_round_trips {
                        while flag.compare_exchange(PONG, PING, Ordering::Relaxed, Ordering::Relaxed).is_err() {}
                    }
                    let end = clock.raw();
                    let duration = clock.delta(start, end).as_nanos();
                    results.push(duration as f64 / num_round_trips as f64 / 2.0);
                }

                results
            });

            pong.join().unwrap();
            ping.join().unwrap()
        }).unwrap()
    }
}
