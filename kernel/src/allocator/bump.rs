use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

use super::{align_up, Locked};

/// A bump allocator that never frees its allocated memory.
///
/// # Fields
///
/// * `heap_start`: The start address of the heap.
/// * `heap_end`: The end address of the heap.
///
/// * `next`: The next free address in the heap.
/// * `allocations`: The number of allocations made.
#[allow(clippy::module_name_repetitions)]
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,

    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Creates a new empty bump allocator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,

            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds.
    ///
    /// # Safety
    /// * This method is unsafe because the caller must ensure that the given memory range is unused.
    /// * Also, this method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;

        self.next = heap_start;
    }
}

/// A global bump allocator instance.
///
/// # Safety
/// * This allocator is not thread safe. Therefore, it must only be used in single-threaded contexts.
unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    /// Allocates memory using the bump allocator.
    ///
    /// # Arguments
    ///
    /// * `layout`: The layout of the memory to allocate.
    ///
    /// # Returns
    ///
    /// * If the allocation succeeds, a pointer to the allocated memory is returned.
    ///
    /// # Safety
    ///
    /// * This function is unsafe because the caller must ensure that the given memory range is unused.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock(); // Get a mutable reference to the allocator.

        let alloc_start = align_up(bump.next, layout.align());
        let Some(alloc_end) = alloc_start.checked_add(layout.size()) else {
            return ptr::null_mut();
        };

        if alloc_end > bump.heap_end {
            ptr::null_mut() // Out of memory!
        } else {
            bump.next = alloc_end;
            bump.allocations += 1;

            alloc_start as *mut u8
        }
    }

    /// Deallocates the memory at the given pointer with the given layout.
    ///
    /// # Arguments
    ///
    /// * `ptr`: The pointer to the memory to deallocate.
    /// * `layout`: The layout of the memory to deallocate.
    ///
    /// # Safety
    ///
    /// * This function is unsafe because the caller must ensure that the given layout is valid.
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Get a mutable reference.
        let mut bump = self.lock();

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}
