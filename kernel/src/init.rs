use crate::errors::Error;
use crate::system::task::executor::Executor;
use crate::KERNEL_VERSION;
use crate::{memory, println};
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
    use crate::allocator;
    use crate::memory::BootInfoFrameAllocator;

    use x86_64::VirtAddr;

    println!("[INFO]: Initializing kernel v{KERNEL_VERSION}...");

    // Initialize the global descriptor table.
    println!("[INFO]: Setting up the GDT...");
    crate::gdt::init();

    // Initialize the interrupt descriptor table.
    println!("[INFO]: Setting up the IDT...");
    crate::interrupts::init_idt();

    // Initialize the programmable interrupt controller.
    println!("[INFO]: Setting up the PIC...");
    unsafe { crate::interrupts::PICS.lock().initialize() };

    // Enable interrupts.
    println!("[INFO]: Enabling interrupts...");
    x86_64::instructions::interrupts::enable();

    // Initialize the memory management.
    println!("[INFO]: Setting up memory management...");
    memory::init(boot_info)?;

    // Initialize the task executor.
    println!("[INFO]: Setting up the task executor...");

    Ok(Executor::new())
}
