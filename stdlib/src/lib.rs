#![no_std]

extern crate alloc;

use alloc::ffi::CString;
use alloc::string::String;
use core::ffi::CStr;
use kernel::print;

/// Prints a string to the VGA text buffer.
///
/// # Arguments
///
/// * `s` - The string to print.
///
/// # Returns
///
/// * `i32` - Zero if the string was printed successfully, otherwise one.
///
/// # Panics
///
/// * If the string is not valid UTF-8.
/// * If the string is not null-terminated.
/// * If the string is not aligned to a byte boundary.
///
/// # Safety
///
/// * The string must be valid UTF-8.
/// * The string must be null-terminated.
/// * The string must be aligned to a byte boundary.
/// * The string must be a valid pointer.
#[allow(clippy::expect_used)]
#[no_mangle]
pub unsafe extern "C" fn print(s: *const i8) -> i32 {
    let s = unsafe { CStr::from_ptr(s) };
    let s = s.to_str().expect("CStr::to_str failed!");

    print!("{s}");

    0
}

/// Reads a line from the keyboard.
///
/// # Returns
///
/// * `*const i8` - The line read from the keyboard.
///
/// # Panics
///
/// * If the waker is already set.
/// * If the waker is not set.
#[allow(clippy::expect_used)]
#[no_mangle]
pub extern "C" fn read_line() -> *const i8 {
    let mut input = String::new();
    kernel::task::keyboard::read_line_blocking(&mut input);

    let input = CString::new(input).expect("CString::new failed!");

    input.into_raw()
}

/// Shuts down the computer.
///
/// # Safety
///
/// * The computer will shut down.
/// * The computer will not reboot.
pub extern "C" fn shutdown() {
    kernel::interrupts::shutdown_interrupt_handler();
}

/// Reboots the computer.
///
/// # Safety
///
/// * The computer will reboot.
pub extern "C" fn reboot() {
    kernel::interrupts::reboot_interrupt_handler();
}
