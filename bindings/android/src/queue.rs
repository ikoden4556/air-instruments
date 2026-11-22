//! A fixed-size single-producer, single-consumer queue of Plucks, from the camera thread
//! (Engine) to the audio callback. Lock-free and allocation-free: a full queue drops the
//! newest Pluck rather than ever blocking either side.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// One Pluck on its way to the synth.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QueuedPluck {
    pub frequency: f32,
    pub velocity: f32,
    /// When the Plucker crossed (or is predicted to), CLOCK_MONOTONIC ns: for latency readouts.
    pub crossing_ns: i64,
}

pub const CAPACITY: usize = 64;

pub struct PluckQueue {
    slots: [UnsafeCell<QueuedPluck>; CAPACITY],
    /// Next slot the consumer reads.
    head: AtomicUsize,
    /// Next slot the producer writes.
    tail: AtomicUsize,
}

// Safety: one producer writes only the slot at `tail` before publishing it; one consumer
// reads only slots before `tail`, then releases them by advancing `head`.
unsafe impl Sync for PluckQueue {}

impl PluckQueue {
    pub const fn new() -> Self {
        Self {
            slots: [const {
                UnsafeCell::new(QueuedPluck {
                    frequency: 0.0,
                    velocity: 0.0,
                    crossing_ns: 0,
                })
            }; CAPACITY],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Producer side. Returns false (and drops the Pluck) when the queue is full.
    pub fn push(&self, pluck: QueuedPluck) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail.wrapping_sub(self.head.load(Ordering::Acquire)) == CAPACITY {
            return false;
        }
        unsafe { *self.slots[tail % CAPACITY].get() = pluck };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Consumer side.
    pub fn pop(&self) -> Option<QueuedPluck> {
        let head = self.head.load(Ordering::Relaxed);
        if head == self.tail.load(Ordering::Acquire) {
            return None;
        }
        let pluck = unsafe { *self.slots[head % CAPACITY].get() };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(pluck)
    }
}

impl Default for PluckQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pluck(i: i64) -> QueuedPluck {
        QueuedPluck {
            frequency: 100.0 + i as f32,
            velocity: 0.5,
            crossing_ns: i,
        }
    }

    #[test]
    fn plucks_come_out_in_the_order_they_went_in() {
        let q = PluckQueue::new();
        for i in 0..3 {
            assert!(q.push(pluck(i)));
        }
        let out: Vec<i64> = std::iter::from_fn(|| q.pop())
            .map(|p| p.crossing_ns)
            .collect();
        assert_eq!(out, [0, 1, 2]);
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn a_full_queue_drops_the_newest_pluck_instead_of_blocking() {
        let q = PluckQueue::new();
        for i in 0..CAPACITY as i64 {
            assert!(q.push(pluck(i)));
        }
        assert!(!q.push(pluck(999)));
        assert_eq!(q.pop().map(|p| p.crossing_ns), Some(0));
        assert!(q.push(pluck(1000)));
        let last = std::iter::from_fn(|| q.pop()).last().map(|p| p.crossing_ns);
        assert_eq!(last, Some(1000));
    }

    #[test]
    fn it_keeps_working_across_many_wraparounds() {
        let q = PluckQueue::new();
        for i in 0..10_000 {
            assert!(q.push(pluck(i)));
            assert_eq!(q.pop().map(|p| p.crossing_ns), Some(i));
        }
    }

    #[test]
    fn a_producer_and_a_consumer_thread_pass_every_pluck_in_order() {
        static Q: PluckQueue = PluckQueue::new();
        const N: i64 = 100_000;
        let producer = std::thread::spawn(|| {
            for i in 0..N {
                while !Q.push(pluck(i)) {
                    std::hint::spin_loop();
                }
            }
        });
        let mut next = 0;
        while next < N {
            if let Some(p) = Q.pop() {
                assert_eq!(p.crossing_ns, next);
                next += 1;
            }
        }
        producer.join().unwrap();
    }
}
