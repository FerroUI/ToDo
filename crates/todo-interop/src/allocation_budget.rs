use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A native parser host can install this allocator globally, then enable its
/// cumulative budget only in a dedicated child process before reading input.
/// Ordinary application allocations take one atomic load and delegate to System.
/// Deallocation never refunds the budget: no headers or pointer tracking are
/// needed, including for allocations made before the budget was enabled.
pub struct ParserAllocator {
    remaining: AtomicUsize,
}

impl ParserAllocator {
    pub const fn new() -> Self {
        Self {
            remaining: AtomicUsize::new(usize::MAX),
        }
    }

    /// Enable once, before parser input or additional threads are created.
    /// Exhaustion makes allocation return null; Rust aborts the isolated child.
    pub fn enable(&self, bytes: usize) -> Result<(), &'static str> {
        if bytes == usize::MAX {
            return Err("The Outlook allocation budget is invalid.");
        }
        self.remaining
            .compare_exchange(usize::MAX, bytes, Ordering::Relaxed, Ordering::Relaxed)
            .map(|_| ())
            .map_err(|_| "The Outlook allocation budget was already enabled.")
    }

    fn reserve(&self, bytes: usize) -> bool {
        let mut remaining = self.remaining.load(Ordering::Relaxed);
        if remaining == usize::MAX {
            return true;
        }
        loop {
            let Some(next) = remaining.checked_sub(bytes) else {
                return false;
            };
            match self.remaining.compare_exchange_weak(
                remaining,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => remaining = actual,
            }
        }
    }
}

impl Default for ParserAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for ParserAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if self.reserve(layout.size()) {
            unsafe { System.alloc(layout) }
        } else {
            std::ptr::null_mut()
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if self.reserve(layout.size()) {
            unsafe { System.alloc_zeroed(layout) }
        } else {
            std::ptr::null_mut()
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Count the full new block even if System could resize it in place.
        // Conservatism keeps all successful decoder allocations within budget.
        if self.reserve(new_size) {
            unsafe { System.realloc(ptr, layout, new_size) }
        } else {
            std::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[cfg(test)]
#[path = "allocation_budget_tests.rs"]
mod tests;
