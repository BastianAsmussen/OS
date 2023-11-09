use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::interrupts;
use x86_64::instructions::port::{Port, PortGeneric, WriteOnlyAccess};
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{gdt, hlt_loop, println};

/// The first PIC offset, used for remapping.
pub const PIC_1_OFFSET: u8 = 32;

/// The second PIC offset, used for remapping.
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// The programmable interrupt controller.
///
/// # Notes
///
/// * This is a spinlock because it is shared between multiple CPUs.
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    /// Convert the interrupt index to a `u8`.
    ///
    /// # Returns
    ///
    /// * `u8` - The interrupt index as a `u8`.
    const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Convert the interrupt index to a `usize`.
    ///
    /// # Returns
    ///
    /// * `usize` - The interrupt index as a `usize`.
    pub(crate) fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

/// Initializes the interrupt descriptor table.
pub fn init_idt() {
    IDT.load();
}

lazy_static! {
    /// The interrupt descriptor table.
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Set the breakpoint handler.
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // Set the double fault handler.
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // Set the page fault handler.
        idt.page_fault.set_handler_fn(page_fault_handler);

        // Add the timer interrupt handler.
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);

        // Add the keyboard interrupt handler.
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);

        idt
    };

    /// The interrupt-request handlers.
    pub static ref INTERRUPT_REQUEST_HANDLERS: Mutex<[fn(); 16]> = Mutex::new([|| {}; 16]);
}

/// Sets the interrupt-request handler for the given interrupt-request index.
///
/// # Arguments
///
/// * `index` - The interrupt-request index.
/// * `handler` - The interrupt-request handler.
///
/// # Safety
///
/// * The interrupt-request handler will be set.
/// * The interrupt-request index must be valid.
/// * The interrupt-request handler must be valid.
pub(crate) fn set_interrupt_request_handler(index: u8, handler: fn()) {
    interrupts::without_interrupts(|| {
        // Get the interrupt handlers.
        let mut handlers = INTERRUPT_REQUEST_HANDLERS.lock();

        handlers[index as usize] = handler;

        // Clear the interrupt mask (enables the interrupt).
        clear_interrupt_mask(index);
    });
}

/// Clears the interrupt mask for the given interrupt request.
///
/// # Arguments
///
/// * `interrupt_request` - The interrupt request.
///
/// # Safety
///
/// * The interrupt mask will be cleared.
/// * The interrupt index must be valid.
fn clear_interrupt_mask(interrupt_request: u8) {
    let (port, ir_value) = if interrupt_request < 8 {
        (0x21, interrupt_request)
    } else {
        (0xA1, interrupt_request - 8)
    };

    let mut port: Port<u8> = Port::new(port);

    unsafe {
        let value = port.read() & !(1 << (ir_value));

        port.write(value);
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Breakpoint Exception!\
        \nStack Frame: {stack_frame:#?}"
    );
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "Double Fault Exception!\
        \nError Code: {error_code}\
        \nStack Frame: {stack_frame:#?}"
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    println!(
        "Page Fault Exception!\
        \nAddress: {:?}\
        \nError Code: {error_code:#?}\
        \nStack Frame: {stack_frame:#?}",
        Cr2::read()
    );

    hlt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::system::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

/// Shuts down the computer.
///
/// # Safety
///
/// * The computer will shut down.
#[no_mangle]
pub extern "C" fn shutdown_interrupt_handler() {
    // Send the shutdown command to the ACPI.
    unsafe {
        let mut port: PortGeneric<u16, WriteOnlyAccess> = PortGeneric::new(0x604);

        port.write(0x2000);
    }
}

/// Reboots the computer.
///
/// # Safety
///
/// * The computer will reboot.
#[no_mangle]
pub extern "C" fn reboot_interrupt_handler() {
    todo!("See https://wiki.osdev.org/Reboot!");
}

#[test_case]
fn test_breakpoint_exception() {
    // Invoke a breakpoint exception.
    interrupts::int3();
}
