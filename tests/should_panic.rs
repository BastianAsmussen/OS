#![no_std]
#![no_main]

use core::panic::PanicInfo;

use kernel::{exit_qemu, serial_print, serial_println, QemuExitCode};

/// The entry point.
///
/// # Returns
///
/// * `!` - Never.
#[allow(clippy::empty_loop)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test did not panic]");

    exit_qemu(QemuExitCode::Failed);

    loop {}
}

fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
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
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[OK]");

    exit_qemu(QemuExitCode::Success);

    loop {}
}
