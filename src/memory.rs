use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    PhysAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator,
        Mapper,
        OffsetPageTable,
        Page,
        page_table::FrameError,
        PageTable,
        PageTableFlags,
        PhysFrame,
        Size4KiB,
    },
    VirtAddr,
};

/// A FrameAllocator that always returns `None`.
pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }
    
    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item=PhysFrame> {
        // Get usable regions from memory map.
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        
        // Map each region to its address range.
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        
        // Transform to an iterator of frame start addresses.
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        
        // Create `PhysFrame` types from the start addresses.
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        
        frame
    }
}

/// Initializes a new OffsetPageTable.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called
/// once to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = activate_level_4_table(physical_memory_offset);
    
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 table.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called
/// once to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn activate_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();
    
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    
    &mut *page_table_ptr // unsafe
}

/// Translates the given virtual address to the mapped physical address,
/// or returns `None` if the address is not mapped.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`.
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
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    
    let map_to_result = unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    
    map_to_result.expect("map_to failed!").flush();
}
