#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(cristian_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use cristian_os::println;

const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Cristian OS booted!");
    println!("Kernel Version: {}", KERNEL_VERSION);
    
    println!("Hva' så bøsser?!");
    
    cristian_os::init();
    
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
    
    #[cfg(test)]
    test_main();
    
    println!("It did not crash!");
    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cristian_os::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
