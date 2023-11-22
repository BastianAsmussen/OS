use crate::println;
use crate::sys::pic::{PICS, PIC_1_OFFSET, PIC_2_OFFSET};
use crate::sys::time::rtc::RTC;
use crate::sys::{gdt, time};
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

        // Set the divide error handler.
        idt.divide_error.set_handler_fn(divide_error_handler);
        // Set the debug handler.
        idt.debug.set_handler_fn(debug_handler);
        // Set the non-maskable interrupt handler.
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        // Set the overflow handler.
        idt.overflow.set_handler_fn(overflow_handler);
        // Set the bound range exceeded handler.
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        // Set the invalid opcode handler.
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        // Set the device not available handler.
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        // Set the double fault handler.
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        // Set the invalid TSS handler.
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        // Set the segment not present handler.
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        // Set the stack segment fault handler.
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        // Set the general protection fault handler.
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        // Set the page fault handler.
        idt.page_fault
            .set_handler_fn(page_fault_handler);
        // Set the x87 floating point handler.
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        // Set the alignment check handler.
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        // Set the machine check handler.
        idt.machine_check.set_handler_fn(machine_check_handler);
        // Set the SIMD floating point handler.
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        // Set the virtualization handler.
        idt.virtualization.set_handler_fn(virtualization_handler);
        // Set the control protection exception handler.
        idt.cp_protection_exception
            .set_handler_fn(cp_protection_exception_handler);
        // Set the hypervisor injection exception handler.
        idt.hv_injection_exception
            .set_handler_fn(hv_injection_exception_handler);
        // Set the VMM (Virtual Machine Monitor) communication exception handler.
        idt.vmm_communication_exception
            .set_handler_fn(vmm_communication_exception_handler);
        // Set the security exception handler.
        idt.security_exception
            .set_handler_fn(security_exception_handler);
        // Set the breakpoint handler.
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // Add the interrupt handlers.
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::RTC.as_usize()].set_handler_fn(rtc_interrupt_handler);

        idt
    };
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Divide Error Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Debug Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Non-Maskable Interrupt Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Overflow Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Bound Range Exceeded Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Invalid Opcode Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Device Not Available Exception!\
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

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!(
        "Invalid TSS Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "Segment Not Present Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "Stack Segment Fault Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "General Protection Fault Exception!\
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
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    println!(
        "x87 Floating Point Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "Alignment Check Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    panic!(
        "Machine Check Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    println!(
        "SIMD Floating Point Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Virtualization Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn cp_protection_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "Control Protection Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn hv_injection_exception_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Hypervisor Injection Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
}

extern "x86-interrupt" fn vmm_communication_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "VMM Communication Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn security_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!(
        "Security Exception!\
        \nError Code: {code}\
        \nStack Frame: {frame:#?}",
        code = error_code,
        frame = stack_frame
    );
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!(
        "Breakpoint Exception!\
        \nStack Frame: {frame:#?}",
        frame = stack_frame
    );
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

    // crate::sys::task::clock::print(&RTC::new_no_check());
}

#[test_case]
fn test_breakpoint_exception() {
    // Invoke a breakpoint exception.
    x86_64::instructions::interrupts::int3();
}
