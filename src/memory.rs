use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        page_table::FrameError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

/// A `FrameAllocator` that always returns `None`.
pub struct EmptyFrameAllocator;

/// A `FrameAllocator` that always returns `None`.
///
/// # Safety
///
/// * This struct is unsafe because the caller must guarantee that the passed memory map is valid. The main requirement is that all frames that are marked as `USABLE` in it are really unused.
unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    /// Allocates a frame.
    ///
    /// # Returns
    ///
    /// * `None` - Always returns `None`.
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

/// A `FrameAllocator` that returns usable frames from the bootloader's memory map.
///
/// # Fields
///
/// * `memory_map`: The memory map passed from the bootloader.
/// * `next`: The index of the next `memory_map` entry to use.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a `FrameAllocator` from the passed memory map.
    ///
    /// # Safety
    /// * This function is unsafe because the caller must guarantee that the passed memory map is valid. The main requirement is that all frames that are marke as `USABLE` in it are really unused.
    #[must_use]
    pub const unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        Self {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    ///
    /// # Returns
    ///
    /// * `impl Iterator<Item = PhysFrame>` - An iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // Get usable regions from memory map.
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);

        // Map each region to its address range.
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());

        // Transform to an iterator of frame start addresses.
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        // Create `PhysFrame` types from the start addresses.
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

/// A `FrameAllocator` that returns usable frames from the bootloader's memory map.
///
/// # Safety
///
/// * This struct is unsafe because the caller must guarantee that the passed memory map is valid. The main requirement is that all frames that are marked as `USABLE` in it are really unused.
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    /// Allocates a frame.
    ///
    /// # Returns
    ///
    /// * `Some(PhysFrame)` - If a free frame was found.
    /// * `None` - If no free frame could be found.
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;

        frame
    }
}

/// Initializes a new `OffsetPageTable`.
///
/// # Arguments
///
/// * `physical_memory_offset`: The offset between physical and virtual memory.
///
/// # Returns
///
/// * `OffsetPageTable<'static>` - A new `OffsetPageTable`.
///
/// # Safety
/// * This function is unsafe because the caller must guarantee that the complete physical memory is mapped to virtual memory at the passed `physical_memory_offset`. Also, this function must be only called once to avoid aliasing `&mut` references (which is undefined behavior).
#[must_use]
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = activate_level_4_table(physical_memory_offset);

    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table.
///
/// # Arguments
///
/// * `physical_memory_offset`: The offset between physical and virtual memory.
///
/// # Returns
///
/// * `&'static mut PageTable` - A mutable reference to the active level 4 table.
///
/// # Safety
/// * This function is unsafe because the caller must guarantee that the complete physical memory is mapped to virtual memory at the passed `physical_memory_offset`. Also, this function must be only called once to avoid aliasing `&mut` references (which is undefined behavior).
#[must_use]
pub unsafe fn activate_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let phys_addr = level_4_table_frame.start_address();
    let virt_addr = physical_memory_offset + phys_addr.as_u64();
    let page_table_ptr: *mut PageTable = virt_addr.as_mut_ptr();

    &mut *page_table_ptr // Unsafe!
}

/// Translates the given virtual address to the mapped physical address,
/// or returns `None` if the address is not mapped.
///
/// # Arguments
///
/// * `addr`: The virtual address to translate.
/// * `physical_memory_offset`: The offset between physical and virtual memory.
///
/// # Returns
///
/// * `Option<PhysAddr>` - The mapped physical address, or `None` if the address is not mapped.
///
/// # Safety
/// * This function is unsafe because the caller must guarantee that the complete physical memory is mapped to virtual memory at the passed `physical_memory_offset`.
///
/// # Panics
///
/// * This function panics if the translation results in an unmapped frame.
#[must_use]
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];

    let mut frame = level_4_table_frame;

    // Walk the page table hierarchy.
    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = &*table_ptr;

        // Get the frame containing the next level of the table.
        let entry = &table[index];

        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("Huge pages not supported!"),
        };
    }

    // Calculate the address by adding the page offset.
    Some(frame.start_address() + u64::from(addr.page_offset()))
}

/// Creates an example mapping for the given page to frame '0xb8000'.
///
/// # Arguments
///
/// * `page`: The page to map.
/// * `mapper`: A mutable reference to the active mapper.
/// * `frame_allocator`: A mutable reference to the active frame allocator.
///
/// # Safety
///
/// * This function is unsafe because the caller must guarantee that the frame is unused.
/// * Also, this function must be only called once to avoid aliasing `&mut` references (which is undefined behavior).
///
/// # Panics
///
/// * This function panics if the mapping fails.
/// * This function panics if the frame is already mapped to another page.
/// * This function panics if the frame is already mapped to the same page.
/// * This function panics if the frame is not page aligned.
#[allow(clippy::expect_used)]
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000)); // The VGA buffer page frame.
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    let map_to_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };

    map_to_result.expect("map_to failed!").flush();
}
