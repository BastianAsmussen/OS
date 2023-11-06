#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};

use kernel::allocator::HEAP_SIZE;

entry_point!(main);

/// Test the heap allocator.
///
/// # Arguments
///
/// * `boot_info` - The boot information.
#[allow(clippy::expect_used, clippy::empty_loop)]
fn main(boot_info: &'static BootInfo) -> ! {
    use kernel::allocator;
    use kernel::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    kernel::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed!");

    test_main();

    loop {}
}

/// This function is called on panic.
///
/// # Arguments
///
/// * `info` - The panic information.
///
/// # Returns
///
/// * `!` - Never.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

/// Tests simple heap allocations.
///
/// # Panics
///
/// * If the heap allocation fails.
#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);

    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}

/// Tests large heap allocations.
///
/// # Panics
///
/// * If the heap allocation fails.
#[test_case]
fn large_vec() {
    let n = 1_000;
    let mut vec = Vec::new();

    for i in 0..n {
        vec.push(i);
    }

    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

/// Tests many heap allocations.
///
/// # Panics
///
/// * If the heap allocation fails.
#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);

        assert_eq!(*x, i);
    }
}

/// Tests many heap allocations with a long-lived reference.
///
/// # Panics
///
/// * If the heap allocation fails.
/// * If the long-lived reference is not equal to the expected value.
#[test_case]
fn many_boxes_long_lived() {
    let long_lived = Box::new(1);
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }

    assert_eq!(*long_lived, 1);
}
