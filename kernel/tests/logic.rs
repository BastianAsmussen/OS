#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::vec::Vec;
use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};

use kernel::task::primes::is_prime;

entry_point!(main);

/// Entry point for `cargo test`.
///
/// # Arguments
///
/// * `boot_info` - The boot information.
///
/// # Returns
///
/// * `!` - Never.
///
/// # Panics
///
/// * If the heap initialization fails.
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

/// Generates a vector of prime numbers.
///
/// # Panics
///
/// * If the length of the vector is not the expected value.
#[test_case]
fn test_primes() {
    const LIMIT: u32 = 1_000;
    const EXPECTED: u32 = 170;

    let mut vec = Vec::new();

    // Loop for 1_000 times, and push the number i into the vector if it is prime.
    (0..LIMIT).filter(|&i| is_prime(i)).for_each(|i| {
        vec.push(i);
    });

    // From 0 to 1_000, there are 168 primes.
    assert_eq!(vec.len(), EXPECTED as usize);
}
