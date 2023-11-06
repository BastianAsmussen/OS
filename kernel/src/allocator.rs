use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use fixed_size_block::FixedSizeBlockAllocator;

pub mod bump;
mod fixed_size_block;
pub mod linked_list;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called!")
    }
}

/// Initialize the heap allocator with the given heap bounds.
///
/// # Arguments
///
/// * `mapper` - A mutable reference to the active mapper.
/// * `frame_allocator` - A mutable reference to the active frame allocator.
///
/// # Returns
///
/// * `Result<(), MapToError<Size4KiB>>` - A result indicating whether the heap initialization succeeded or failed.
///
/// # Safety
///
/// * The caller must guarantee that the given heap bounds are valid and that the heap is unused.
/// * This method must be called only once.
///
/// # Errors
///
/// * `MapToError<Size4KiB>` - Indicates that the heap initialization failed.
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    // Create a page range containing the heap pages.
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64; // Subtract 1 because the range is inclusive.

        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);

        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Map the heap pages.
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    // Initialize the heap allocator. This is safe because we mapped the heap pages.
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    // Return the heap allocator.
    Ok(())
}

/// A wrapper around `spin::Mutex` to permit trait implementations.
///
/// # Type Parameters
///
/// * `A` - The type to wrap.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    /// Create a new `Locked` wrapper around the given value.
    ///
    /// # Arguments
    ///
    /// * `inner` - The value to wrap.
    pub const fn new(inner: A) -> Self {
        Self {
            inner: spin::Mutex::new(inner),
        }
    }

    /// Lock the inner value.
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
///
/// # Arguments
///
/// * `addr` - The address to align.
/// * `align` - The alignment to use.
///
/// # Returns
///
/// * `usize` - The aligned address.
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
