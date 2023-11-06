#![no_std]

use kernel::println;

/// The type of shutdown to perform.
///
/// # Values
///
/// * `Reboot` - Reboot the computer.
/// * `Shutdown` - Shutdown the computer.
#[derive(Debug, Clone, Copy)]
enum ShutdownType {
    Reboot,
    Shutdown,
}

/// Handles the `shutdown` command.
///
/// # Arguments
///
/// * `args` - The arguments to the `shutdown` command.
pub fn run(args: &[&str]) {
    let shutdown_type = match args.first() {
        Some(&"-r") => ShutdownType::Reboot,
        Some(&"-s") | None => ShutdownType::Shutdown,
        _ => {
            println!("Usage: shutdown [-r | -s]");

            return;
        }
    };

    match shutdown_type {
        ShutdownType::Reboot => {
            println!("Rebooting...");
            stdlib::reboot();
        }
        ShutdownType::Shutdown => {
            println!("Shutting down...");
            stdlib::shutdown();
        }
    }
}
