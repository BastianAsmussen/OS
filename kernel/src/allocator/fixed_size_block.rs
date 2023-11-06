use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::{mem, ptr};

use crate::allocator::Locked;

/// The block sizes to use.
///
/// The sizes must each be power of 2 because they are also used as
/// the block alignment (alignments must be always powers of 2).
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// A node in the linked list.
///
/// # Fields
///
/// * `next`: A reference to the next node in the linked list.
struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// A fixed size block allocator.
///
/// # Fields
///
/// * `list_heads`: The heads of the linked lists.
/// * `fallback_allocator`: The fallback allocator.
#[allow(clippy::module_name_repetitions)]
pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// Creates an empty `FixedSizeBlockAllocator`.
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;

        Self {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// # Safety
    /// * This function is unsafe because the caller must guarantee that the given heap bounds are valid and that the heap is unused.
    /// * This method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        let heap_bottom = heap_start as *mut u8;

        self.fallback_allocator.init(heap_bottom, heap_size);
    }

    /// Allocates using the fallback allocator.
    ///
    /// # Arguments
    ///
    /// * `layout`: The layout of the memory to allocate.
    ///
    /// # Returns
    ///
    /// * `*mut u8` - A pointer to the allocated memory.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        self.fallback_allocator
            .allocate_first_fit(layout)
            .ok()
            .map_or(ptr::null_mut(), NonNull::as_ptr)
    }
}

/// Choose an appropriate block size for the given layout.
///
/// # Arguments
///
/// * `layout` - The layout of the memory to allocate.
///
/// # Returns
///
/// * `Option<usize>` - The index of the block size to use.
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());

    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

/// A global fixed size block allocator instance.
unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    /// Allocates memory using the fixed size block allocator.
    ///
    /// # Arguments
    ///
    /// * `layout` - The layout of the memory to allocate.
    ///
    /// # Returns
    ///
    /// * `*mut u8` - A pointer to the allocated memory.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the given memory range is unused.
    /// * The caller must ensure that the given layout is valid.
    /// * The caller must ensure that the allocation succeeds.
    #[allow(clippy::expect_used)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        match list_index(&layout) {
            Some(index) => {
                if let Some(node) = allocator.list_heads[index].take() {
                    allocator.list_heads[index] = node.next.take();

                    (node as *mut ListNode).cast::<u8>()
                } else {
                    // No block exists in list => allocate new block.
                    let block_size = BLOCK_SIZES[index];

                    // Only works if all block sizes are a power of 2.
                    let block_align = block_size;
                    let layout = Layout::from_size_align(block_size, block_align)
                        .expect("Wrong block size!");

                    allocator.fallback_alloc(layout)
                }
            }
            None => allocator.fallback_alloc(layout),
        }
    }

    /// Deallocates the memory at the given pointer with the given layout.
    ///
    /// # Arguments
    ///
    /// * `ptr` - The pointer to the memory to deallocate.
    /// * `layout` - The layout of the memory to deallocate.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the given layout is valid.
    /// * The caller must ensure that the given pointer is valid.
    /// * The caller must ensure that the given pointer is allocated.
    #[allow(clippy::expect_used, clippy::cast_ptr_alignment)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        if let Some(index) = list_index(&layout) {
            let new_node = ListNode {
                next: allocator.list_heads[index].take(),
            };

            // Verify that block has size and alignment required for storing node.
            assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
            assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

            let new_node_ptr = ptr.cast::<ListNode>();
            new_node_ptr.write(new_node);

            allocator.list_heads[index] = Some(&mut *new_node_ptr);
        } else {
            let ptr = NonNull::new(ptr).expect("Null pointer passed to deallocate!");

            allocator.fallback_allocator.deallocate(ptr, layout);
        }
    }
}
