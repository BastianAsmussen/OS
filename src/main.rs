#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(basic_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use bootloader::{BootInfo, entry_point};

use basic_os::println;

const OS_NAME: &str = "Cristian OS";
const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use basic_os::memory;
    use x86_64::{structures::paging::Page, VirtAddr};
    
    println!("{} v{}", OS_NAME, KERNEL_VERSION);
    basic_os::init();
    
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = memory::EmptyFrameAllocator;
    
    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);
    
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };
    
    #[cfg(test)]
    test_main();
    
    println!("I didn't die!");
    
    basic_os::hlt_loop();
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
