#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(basic_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{BootInfo, entry_point};

use basic_os::{memory, println};
use basic_os::task::{keyboard, Task};
use basic_os::task::executor::Executor;

const OS_NAME: &str = "Cristian OS";
const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use basic_os::allocator;
    use basic_os::memory::BootInfoFrameAllocator;
    use x86_64::VirtAddr;
    
    println!("{} v{}", OS_NAME, KERNEL_VERSION);
    
    // Initialize the GDT, IDT, PIC, and enable interrupts.
    basic_os::init();
    
    // Initialize the memory management.
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    
    // Initialize the heap memory allocator.
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed!");
    
    // Run tests.
    #[cfg(test)]
    test_main();
    
    let mut executor = Executor::new();
    
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypress()));
    executor.run();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    
    basic_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    basic_os::test_panic_handler(info)
}

#[test_case]
#[allow(clippy::eq_op)] // allow trivial assertion
fn trivial_assertion() {
    assert_eq!(1, 1);
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}
