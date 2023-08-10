use cache_padded::CachePadded;
use core_affinity::CoreId;
use std::sync::Barrier;
use std::sync::atomic::{Ordering, AtomicU64};
use quanta::Clock;

use super::Count;
use crate::utils;

#[derive(Default)]
pub struct Bench;

impl super::Bench for Bench {
    // This test is not symmetric. We are doing one-way message passing.
    fn is_symmetric(&self) -> bool { false }

    fn run(
        &self,
        (recv_core, send_core): (CoreId, CoreId),
        clock: &Clock,
        num_iterations: Count,
        num_samples: Count,
    ) -> Vec<f64> {
        // Mind first-touch binds memory to current numa node
        core_affinity::set_for_current(recv_core);

        let clock_read_overhead_sum = utils::clock_read_overhead_sum(clock, num_iterations);

        // A shared time reference
        let start_time = clock.raw();

        // Shared states
        let ref barrier = Barrier::new(2);
        let ref clocks = (0..num_iterations as usize).map(|_| Default::default()).collect::<Vec<CachePadded<AtomicU64>>>();

        crossbeam_utils::thread::scope(|s| {
            let receiver = s.spawn(|_| {
                core_affinity::set_for_current(recv_core);
                let mut results = Vec::with_capacity(num_samples as usize);

                barrier.wait();

                for _ in 0..num_samples as usize {
                    let mut latency: u64 = 0;

                    barrier.wait();
                    for v in clocks {
                        // RDTSC is compensated below
                        let send_time = wait_for_non_zero_value(v, Ordering::Relaxed);
                        let recv_time = clock.raw().saturating_sub(start_time);
                        latency += recv_time.saturating_sub(send_time);
                    }
                    barrier.wait();

                    let total_latency = clock.delta(0, latency).saturating_sub(clock_read_overhead_sum).as_nanos();
                    results.push(total_latency as f64 / num_iterations as f64);
                }

                results
            });

            let sender = s.spawn(|_| {
                core_affinity::set_for_current(send_core);

                barrier.wait();

                for _ in 0..num_samples as usize {
                    barrier.wait();
                    for v in clocks {
                        // Stall a bit to make sure the receiver is ready and we're not getting ahead of ourselves
                        // We could also put a barrier().wait(), but it's unclear whether it's a good
                        // idea due to additional generated traffic.
                        utils::delay_cycles(10000);

                        // max(1) to make sure the value is non-zero, which is what the receiver is waiting on
                        let send_time = clock.raw().saturating_sub(start_time).max(1);
                        v.store(send_time, Ordering::Relaxed);
                    }

                    barrier.wait();
                    for v in clocks {
                        v.store(0, Ordering::Relaxed);
                    }
                }
            });

            sender.join().unwrap();
            receiver.join().unwrap()
        }).unwrap()
    }
}

fn wait_for_non_zero_value(atomic_value: &AtomicU64, ordering: Ordering) -> u64 {
    loop {
        match atomic_value.load(ordering) {
            0 => continue,
            v => return v,
        }
    }
}
