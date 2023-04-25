#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(basic_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::vec::Vec;
use core::panic::PanicInfo;

use bootloader::{BootInfo, entry_point};

use basic_os::task::primes;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use basic_os::allocator;
    use basic_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;
    
    basic_os::init();
    
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed!");
    
    test_main();
    
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    basic_os::test_panic_handler(info)
}

#[test_case]
fn test_primes() {
    const LIMIT: u32 = 1_000;
    const EXPECTED: u32 = 168;
    
    let mut vec = Vec::new();
    
    // Loop for 1_000 times, and push the number i into the vector if it is prime.
    for i in 0..LIMIT {
        
        // If the number is prime, push it into the vector.
        if primes::is_prime(i) {
            vec.push(i);
        }
    }
    
    // From 0 to 1_000, there are 168 primes.
    assert_eq!(vec.len(), EXPECTED as usize);
}
