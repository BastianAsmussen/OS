#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(const_mut_refs)]

extern crate alloc;

use core::panic::PanicInfo;

#[cfg(test)]
use bootloader::{entry_point, BootInfo};

/// The version of the kernel.
pub const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod allocator;
pub mod dev;
pub mod errors;
pub mod fs;
pub mod init;
pub mod mem;
pub mod serial;
pub mod sys;
pub mod vga_buffer;

/// This function is called on panic.
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// The `Testable` trait, which is implemented for test functions.
pub trait Testable {
    /// Runs the test.
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    /// Runs the test.
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());

        self();

        serial_println!("[OK]");
    }
}

/// Runs the given tests.
///
/// # Arguments
///
/// * `tests` - The tests to run.
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests...", tests.len());
    for test in tests {
        test.run();
    }

    exit_qemu(QemuExitCode::Success);
}

/// Called on panic in `cargo test`
///
/// # Arguments
///
/// * `info` - A reference to the panic info.
///
/// # Returns
///
/// * `!` - Never.
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!(
        "[ERROR]\
        \nError: {}",
        info
    );

    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

/// The QEMU exit code.
///
/// This is used to tell QEMU whether the test succeeded or failed.
///
/// # Values
///
/// * `Success` - The test succeeded.
/// * `Failed` - The test failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Exits QEMU with the given exit code.
///
/// # Arguments
///
/// * `exit_code` - The exit code.
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xF4);

        port.write(exit_code as u32);
    }
}

/// Called on panic in `cargo test`.
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
    test_panic_handler(info)
}

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo test`.
///
/// # Arguments
///
/// * `boot_info` - A reference to the boot information.
///
/// # Returns
///
/// * `!` - Never.
#[allow(clippy::no_mangle_with_rust_abi)]
#[cfg(test)]
#[no_mangle]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    init::start_kernel(boot_info).expect("Failed to start kernel!");
    hlt_loop();
}
