use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use arbor::{Constant, Node, Sequence, Status};

struct CountingAllocator {
    enabled: AtomicBool,
    allocations: AtomicUsize,
}

impl CountingAllocator {
    const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            allocations: AtomicUsize::new(0),
        }
    }

    fn enable(&self) {
        self.allocations.store(0, Ordering::SeqCst);
        self.enabled.store(true, Ordering::SeqCst);
    }

    fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    fn count(&self) -> usize {
        self.allocations.load(Ordering::SeqCst)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator::new();

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if self.enabled.load(Ordering::Relaxed) && !ptr.is_null() {
            self.allocations.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let out = unsafe { System.realloc(ptr, layout, new_size) };
        if self.enabled.load(Ordering::Relaxed) && !out.is_null() {
            self.allocations.fetch_add(1, Ordering::Relaxed);
        }
        out
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if self.enabled.load(Ordering::Relaxed) && !ptr.is_null() {
            self.allocations.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }
}

#[tokio::test(flavor = "current_thread")]
async fn hot_tick_loop_allocates_zero_after_construction() {
    let mut tree = Sequence::new((
        Constant::new(Status::Success),
        Constant::new(Status::Success),
        Constant::new(Status::Success),
    ));
    let mut ctx = ();

    // Warm-up to avoid counting one-time runtime/setup allocations.
    assert_eq!(tree.tick(&mut ctx).await, Status::Success);

    ALLOCATOR.enable();

    for _ in 0..500 {
        assert_eq!(tree.tick(&mut ctx).await, Status::Success);
    }

    ALLOCATOR.disable();

    assert_eq!(ALLOCATOR.count(), 0, "tick path performed allocations");
}
