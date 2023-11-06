#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_kernel_main"]
extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};

use kernel::task::executor::Executor;
use kernel::task::{keyboard, Task};
use kernel::{memory, println};

const OS_NAME: &str = "Rust OS";
const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

entry_point!(kernel_main);

/// The kernel main function.
///
/// # Arguments
///
/// * `boot_info` - A reference to the boot information.
///
/// # Returns
///
/// * `!` - Never.
#[allow(clippy::expect_used)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kernel::allocator;
    use kernel::memory::BootInfoFrameAllocator;
    use x86_64::VirtAddr;

    println!("{OS_NAME} v{KERNEL_VERSION}");

    // Initialize the GDT, IDT, PIC, and enable interrupts.
    kernel::init();

    // Initialize the memory management.
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // Initialize the heap memory allocator.
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed!");

    // Run tests.
    #[cfg(test)]
    test_kernel_main();

    println!("Hello, world!");

    let mut executor = Executor::new();

    executor.spawn(Task::new(keyboard::print_keypress()));
    executor.run();
}

/// This function is called on panic.
///
/// # Arguments
///
/// * `info` - A reference to the panic info.
///
/// # Returns
///
/// * `!` - Never.
#[cfg(not(test))]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("{info}");

    kernel::hlt_loop();
}

/// This function is called on panic.
///
/// # Arguments
///
/// * `info` - A reference to the panic info.
///
/// # Returns
///
/// * `!` - Never.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}
