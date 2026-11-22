//! Plucking and rendering run on the audio thread, so they must never allocate.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use air_core::synth::Synth;

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

#[test]
fn plucking_and_rendering_do_not_allocate() {
    let before_new = ALLOCATIONS.load(Ordering::SeqCst);
    let mut synth = Synth::new(48_000.0);
    let mut out = vec![0.0; 128];
    // The counter works: making the Synth does allocate.
    assert!(ALLOCATIONS.load(Ordering::SeqCst) > before_new);

    let before = ALLOCATIONS.load(Ordering::SeqCst);
    for i in 0..40 {
        synth.pluck(110.0 + 20.0 * i as f32, 0.8);
        synth.render(&mut out);
    }
    assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), before);
}
