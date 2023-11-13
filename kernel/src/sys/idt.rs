use crate::sys::pic::{PICS, PIC_1_OFFSET, PIC_2_OFFSET};
use crate::sys::time::rtc::RTC;
use crate::sys::{gdt, time};
use crate::{hlt_loop, println};
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

/// The interrupt indices.
///
/// # Variants
///
/// 1. `Timer` - The timer interrupt (exists at [`PIC_1_OFFSET`]).
/// 2. `Keyboard` - The keyboard interrupt, used for keyboard input (exists at [`PIC_1_OFFSET`] + 1).
/// 3. `RTC` - The RTC interrupt, used for the RTC (exists at [`PIC_2_OFFSET`]).
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    RTC = PIC_2_OFFSET,
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
    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

/// Initializes the interrupt descriptor table.
pub fn init() {
    IDT.load();
}

lazy_static! {
    /// The interrupt descriptor table.
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Set the breakpoint handler.
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        unsafe {
            // Set the double fault handler.
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

            // Set the page fault handler.
            idt.page_fault
                .set_handler_fn(page_fault_handler);
        };

        // Add the interrupt handlers.
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::RTC.as_usize()].set_handler_fn(rtc_interrupt_handler);

        idt
    };
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Breakpoint Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "Double Fault Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    println!(
        "Page Fault Exception!\
        \nAddress: {addr:?}\
        \nError Code: {code:#?}\
        \nStack Frame: {frame:#?}",
        addr = Cr2::read(),
        code = error_code,
        frame = stack_frame
    );

    hlt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Increment the PIT tick.
    time::PIT_TICK.fetch_add(1, Ordering::Relaxed);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::sys::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn rtc_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Store the last RTC update tick.
    time::LAST_RTC_UPDATE.store(time::tick(), Ordering::Relaxed);

    // Notify the RTC that the interrupt has ended.
    RTC::default().notify_interrupt_end();

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::RTC.as_u8());
    }

    crate::sys::task::clock::print_clock(&RTC::new_no_check());
}

#[test_case]
fn test_breakpoint_exception() {
    // Invoke a breakpoint exception.
    x86_64::instructions::interrupts::int3();
}
