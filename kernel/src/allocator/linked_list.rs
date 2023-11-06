use core::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};

use crate::allocator::Locked;

use super::align_up;

/// A node in the linked list.
///
/// # Fields
///
/// * `size`: The size of the memory region in bytes.
/// * `next`: A reference to the next node in the linked list.
struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        Self { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// A linked list allocator.
///
/// # Fields
///
/// * `head`: The head of the linked list.
#[allow(clippy::module_name_repetitions)]
pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// `LinkedListAllocator` is a linked list of free memory blocks.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// Adds the given memory region to the front of the list.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the given
    /// memory region is unused.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // Ensure that the freed region is capable of holding ListNode.
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // Create a new list node and append it at the start of the list.
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();

        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);

        self.head.next = Some(&mut *node_ptr);
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// Adds the given memory region to the front of the list.
    ///
    /// # Safety
    /// * This function is unsafe because the caller must guarantee that the given memory region is unused.
    /// * Try to use the given region for an allocation with given size and alignment.
    ///
    /// # Returns
    ///
    /// * `Result<usize, ()>`: If the allocation succeeds, then the start address.
    /// * `Err(())`: If the allocation fails.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // Region is too small!
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // The rest of region too small to hold a ListNode (required because the allocation splits the region in a used and a free part).
            return Err(());
        }

        // Region suitable for allocation.
        Ok(alloc_start)
    }

    /// Looks for a free region with the given size and alignment and removes
    /// it from the list.
    ///
    /// # Returns
    ///
    /// * `Some((addr, alloc_start))`: If a suitable region was found, then
    /// * `None`: If no suitable region was found.
    ///
    /// # Panics
    ///
    /// * If adjusting the alignment fails.
    #[allow(clippy::expect_used)]
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // Reference to current list node, updated for each iteration.
        let mut current = &mut self.head;

        // Look for a large enough memory region in linked list.
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(region, size, align) {
                // Region suitable for allocation -> remove node from list.
                let next = region.next.take();
                let ret = Some((
                    current.next.take().expect("Expected next region!"),
                    alloc_start,
                ));

                current.next = next;

                return ret;
            }

            // Region not suitable -> continue with next region.
            current = current.next.as_mut().expect("Expected next region!");
        }

        // No suitable region found.
        None
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `ListNode`.
    ///
    /// # Returns
    /// * `(usize, usize)`: The adjusted size and alignment as a (size, align) tuple.
    ///
    /// # Panics
    ///
    /// * If adjusting the alignment fails due to overflow.
    #[allow(clippy::expect_used)]
    #[must_use]
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("Adjusting alignment failed!")
            .pad_to_align();

        let size = layout.size().max(mem::size_of::<ListNode>());

        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    /// Allocate memory with the given layout.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the given layout is valid.
    /// * The caller must ensure that the allocation succeeds.
    ///
    /// # Returns
    ///
    /// * `*mut u8`: Pointer to the allocated memory.
    #[allow(clippy::expect_used)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Perform layout adjustments.
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        // Look for a suitable region and allocate it.
        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start
                .checked_add(size)
                .expect("Allocation failed due to overflow!");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.find_region(alloc_end, excess_size);
            }

            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    /// Deallocate the memory region at the given pointer with the given layout.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the given layout is valid.
    /// * The caller must ensure that the given pointer is valid.
    /// * The caller must ensure that the given memory region is not used anymore.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Perform layout adjustments.
        let (size, _) = LinkedListAllocator::size_align(layout);

        // Add freed region to the list.
        self.lock().add_free_region(ptr as usize, size);
    }
}
