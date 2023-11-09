use crate::cmos::RTC;
use crate::errors::Error;
use crate::system::task::executor::Executor;
use crate::system::task::{keyboard, Task};
use crate::time::sleep;
use crate::{memory, println};
use crate::{time, KERNEL_VERSION};
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

    // Initialize the PIT.
    println!("[INFO]: Setting up the PIT...");
    time::init()?;

    // Enable interrupts.
    println!("[INFO]: Enabling interrupts...");
    x86_64::instructions::interrupts::enable();

    // Initialize the memory management.
    println!("[INFO]: Setting up memory management...");
    memory::init(boot_info)?;

    // Initialize the task executor.
    println!("[INFO]: Setting up the task executor...");
    let mut executor = Executor::new();

    executor.spawn(Task::new(keyboard::print_keypress()))?;
    executor.spawn(Task::new(async {
        loop {
            let time = RTC::new();
            println!("Time: {time:#?}");
            // TODO: Figure out why it's so off.
            let multiplier = 100.0; // 100 = 6 seconds.
            let secs_to_sleep = 3.0; // 3 seconds.
            let seconds = secs_to_sleep * multiplier; // 3 * 100 = 300 which is 18 seconds.
            sleep(seconds);
        }
    }))?;

    Ok(executor)
}
