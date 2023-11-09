use crate::cmos::{RTCInterrupt, RTC};
use crate::errors::Error;
use crate::pit::{AccessMode, Channel, OperatingMode};
use crate::{clock, println};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use x86_64::instructions::interrupts;
use x86_64::instructions::port::Port;

/// The frequency of the PIT, in Hz.
const PIT_FREQUENCY: f64 = 3_579_545.0 / 3.0;

/// The frequency divider for the PIT, in Hz.
const PIT_DIVIDER: f64 = 1_193_182.0;

/// The interval between PIT ticks, in seconds.
const PIT_INTERVAL: f64 = PIT_DIVIDER / PIT_FREQUENCY;

/// The current tick of the PIT.
static CURRENT_PIT_TICK: AtomicUsize = AtomicUsize::new(0);
/// The last RTC update tick.
static LAST_RTC_UPDATE: AtomicUsize = AtomicUsize::new(0);
/// The clock cycles per nanosecond.
static CLOCK_CYCLES_PER_NS: AtomicU64 = AtomicU64::new(0);

/// Ticks the PIT.
///
/// # Returns
///
/// * `usize` - The current tick.
pub fn tick() -> usize {
    CURRENT_PIT_TICK.load(Ordering::Relaxed)
}

/// Gets the PIT interval.
///
/// # Returns
///
/// * `f64` - The PIT interval in seconds.
#[must_use]
pub const fn interval() -> f64 {
    PIT_INTERVAL
}

/// Puts the CPU to sleep.
pub fn halt() {
    // Save the state of the interrupts.
    let was_disabled = !interrupts::are_enabled();

    // Enable interrupts, and halt the CPU.
    interrupts::enable_and_hlt();

    // Restore the state of the interrupts.
    if was_disabled {
        interrupts::disable();
    }
}

/// Sleeps for the given amount of seconds.
///
/// # Arguments
///
/// * `seconds` - The amount of seconds.
pub fn sleep(seconds: f64) {
    let start = clock::uptime();

    while clock::uptime() - start < seconds {
        halt();
    }
}

/// Sets the PIT frequency divider.
///
/// # Arguments
///
/// * `divider` - The frequency divider.
/// * `channel` - The PIT channel.
///
/// # Returns
///
/// * `Result<(), Error>` - The result of the operation.
///
/// # Errors
///
/// * If the channel is not supported.
fn set_pit_freq_divider(divider: u16, channel: &Channel) -> Result<(), Error> {
    let channel = match channel {
        Channel::Channel0 => u8::from(Channel::Channel0),
        _ => return Err(Error::Internal("Unsupported PIT channel!".into())),
    };

    let channel = 0;
    let operating_mode = 6;
    let access_mode = 3;

    interrupts::without_interrupts(|| {
        let bytes = divider.to_le_bytes();

        let mut addr: Port<u8> = Port::new(0x43);
        let mut data: Port<u8> = Port::new(0x40 + u16::from(channel));

        unsafe {
            // Write the PIT mode.
            addr.write((channel << 6) | (access_mode << 4) | operating_mode);
            // Write the 2 bytes of the divider.
            data.write(bytes[0]);
            data.write(bytes[1]);
        }
    });

    Ok(())
}

/// The PIT interrupt handler.
pub fn pit_interrupt_handler() {
    // Increment the PIT tick.
    CURRENT_PIT_TICK.fetch_add(1, Ordering::Relaxed);
}

/// The RTC interrupt handler.
pub fn rtc_interrupt_handler() {
    // Update the last RTC update tick.
    LAST_RTC_UPDATE.fetch_add(tick(), Ordering::Relaxed);

    // Notify the RTC that the interrupt has ended.
    RTC::default().notify_interrupt_end();
}

/// Initializes the PIT.
///
/// # Returns
///
/// * `Result<(), Error>` - The result of the initialization.
///
/// # Errors
///
/// * If the PIT frequency divider cannot be converted to a `u16`.
/// * If the PIT channel is not supported.
///
/// # Panics
///
/// * If the PIT channel is not supported.
pub fn init() -> Result<(), Error> {
    // Check if the PIT divider is too large.
    let lowest_divider: usize = 65_536;
    let divider = if PIT_DIVIDER < lowest_divider as f64 {
        PIT_DIVIDER as u16
    } else {
        0
    };

    // Set the PIT frequency divider.
    set_pit_freq_divider(divider, &Channel::Channel0)?;
    crate::interrupts::set_interrupt_request_handler(0, pit_interrupt_handler);

    // Set the RTC interrupt handler.
    crate::interrupts::set_interrupt_request_handler(8, rtc_interrupt_handler);
    RTC::default().set_interrupt(&RTCInterrupt::Update, true);

    // Set calibration values.
    let calib = 250_000;
    let a = tsc();

    // Sleep for ~0.25 seconds
    sleep(calib as f64 / 1e6);

    let b = tsc();

    // Calculate the clock cycles per nanosecond.
    CLOCK_CYCLES_PER_NS.store((b - a) / calib, Ordering::Relaxed);

    Ok(())
}

/// Reads the time-stamp counter.
///
/// # Returns
///
/// * `u64` - The time-stamp counter.
fn tsc() -> u64 {
    unsafe {
        core::arch::x86_64::_mm_lfence();
        core::arch::x86_64::_rdtsc()
    }
}

/// Measures the time of a function.
///
/// # Arguments
///
/// * `function` - The function to measure.
///
/// # Returns
///
/// * `(u64, R)` - The time in cycles and the result of the function.
fn time<F, R>(function: F) -> (u64, R)
where
    F: FnOnce() -> R,
{
    let start = tsc();
    let result = function();
    let end = tsc();

    (end - start, result)
}
