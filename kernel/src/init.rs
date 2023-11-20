use crate::errors::Error;
use crate::sys::task::executor::Executor;
use crate::sys::task::{keyboard, Task};
use crate::sys::{gdt, idt, pic, time};
use crate::{dev, fs, KERNEL_VERSION};
use crate::{mem, println};
use bootloader::BootInfo;

/// Initializes the kernel.
///
/// # Arguments
///
/// * `boot_info` - A reference to the boot information.
///
/// # Returns
///
/// * `Result<Executor, anyhow::Error>` - The executor.
///
/// # Errors
///
/// * If the heap memory allocator fails to initialize.
pub fn start_kernel(boot_info: &'static BootInfo) -> Result<Executor, Error> {
    println!(
        "[INFO]: Initializing kernel v{version}...",
        version = KERNEL_VERSION
    );

    // Initialize the global descriptor table.
    println!("[INFO]: Configuring GDT...");
    gdt::init();

    // Initialize the interrupt descriptor table.
    println!("[INFO]: Configuring IDT...");
    idt::init();

    // Initialize the programmable interrupt controller.
    println!("[INFO]: Configuring PIC...");
    unsafe { pic::PICS.lock().initialize() };

    // Enable interrupts.
    println!("[INFO]: Enabling interrupts...");
    x86_64::instructions::interrupts::enable();

    // Initialize the PIT.
    println!("[INFO]: Configuring PIT...");
    time::init()?;

    // Initialize the memory management.
    println!("[INFO]: Configuring memory management...");
    mem::init(boot_info)?;

    // Initialize the device drivers.
    println!("[INFO]: Initializing device drivers...");
    dev::init();

    // Initialize the file system.
    println!("[INFO]: Initializing the file system...");
    fs::init();

    // Initialize the task executor.
    println!("[INFO]: Setting up the task executor...");
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypress()))?;

    Ok(executor)
}
