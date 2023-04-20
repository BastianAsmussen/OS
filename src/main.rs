#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(basic_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use basic_os::println;

const OS_NAME: &str = "Cristian OS";
const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[no_mangle]
pub extern "C" fn _start() -> ! {
    
    println!("Initializing {}...", OS_NAME);
    
    // Initialize the kernel, and then run the tests.
    basic_os::init();
    
    // Print the time it took to initialize the kernel.
    println!("Initialized in {} ms!", -1);
    println!("Kernel Version: {}", KERNEL_VERSION);
    
    #[cfg(test)]
    test_main();
    
    println!("It did not crash!");
    
    // Infinite loop to prevent the kernel from exiting.
    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    
    loop {}
}

/// This function is for testing the panic handler.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    basic_os::test_panic_handler(info)
}
