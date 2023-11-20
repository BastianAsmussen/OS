pub mod clock;
pub mod cmos;
pub mod rtc;

use core::hint::spin_loop;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use x86_64::instructions::{interrupts, port::Port};

use crate::errors::Error;
use crate::sys::pit::{AccessMode, Channel, OperatingMode};
use crate::sys::time::rtc::{RTCInterrupt, RTC};

/// The PIT divider, used to calculate the PIT frequency by dividing the PIT clock frequency, in Hz.
const PIT_DIVIDER: usize = 1_193;

/// The PIT frequency, in Hz.
pub const PIT_FREQUENCY: f64 = 3_579_545.0 / 3.0;

/// The PIT interval, in seconds, between each PIT tick.
const PIT_INTERVAL: f64 = (PIT_DIVIDER as f64) / PIT_FREQUENCY;

/// The current PIT tick.
pub(crate) static PIT_TICK: AtomicUsize = AtomicUsize::new(0);

/// The last RTC update, in PIT ticks.
pub(crate) static LAST_RTC_UPDATE: AtomicUsize = AtomicUsize::new(0);

/// The number of clock cycles per nanosecond.
static CLOCK_CYCLES_PER_NS: AtomicU64 = AtomicU64::new(0);

/// Gets the last PIT tick.
///
/// # Returns
///
/// * `usize` - The last PIT tick.
pub fn tick() -> usize {
    PIT_TICK.load(Ordering::Relaxed)
}

/// Gets the time between each PIT tick.
///
/// # Returns
///
/// * `f64` - The PIT interval.
#[must_use]
pub const fn pit_interval() -> f64 {
    PIT_INTERVAL
}

/// Gets the last RTC update.
///
/// # Returns
///
/// * `usize` - The last RTC update.
pub fn last_rtc_update() -> usize {
    LAST_RTC_UPDATE.load(Ordering::Relaxed)
}

/// Halt the CPU until the next interrupt.
/// It will enable interrupts if they were disabled before halting, and disable them again before returning.
pub fn halt() {
    let was_disabled = !interrupts::are_enabled();

    interrupts::enable_and_hlt();

    if was_disabled {
        interrupts::disable();
    }
}

/// Initializes the time-keeping functionality.
///
/// # Returns
///
/// * `Result<(), Error>` - The result of the operation.
///
/// # Errors
///
/// * If the PIT frequency divider is invalid.
pub fn init() -> Result<(), Error> {
    // Set the PIT frequency divider.
    let divider = if PIT_DIVIDER < 65_536 { PIT_DIVIDER } else { 0 };
    set_pit_frequency_divider(u16::try_from(divider)?, &Channel::Zero)?;

    // Enable the RTC Update interrupt.
    RTC::default().set_interrupt(&RTCInterrupt::Update, true);

    // Calibrate the clock.
    let calibration = 250_000;

    let start = read_tsc();
    sleep(calibration as f64 / 1e6); // Sleeps for 0.25 ms.
    let end = read_tsc();

    CLOCK_CYCLES_PER_NS.store((end - start) / calibration, Ordering::Relaxed);

    Ok(())
}

/// Reads the time-stamp counter.
///
/// # Returns
///
/// * `u64` - The time-stamp counter.
fn read_tsc() -> u64 {
    unsafe {
        core::arch::x86_64::_mm_lfence(); // Prevents instruction reordering.
        core::arch::x86_64::_rdtsc() // Reads the time-stamp counter.
    }
}

/// Sleeps for the given amount of seconds.
///
/// # Arguments
///
/// * `seconds` - The amount of seconds to sleep.
pub fn sleep(seconds: f64) {
    let start = clock::uptime();

    while clock::uptime() - start < seconds {
        halt();
    }
}

/// Waits for the given amount of nanoseconds.
///
/// # Arguments
///
/// * `ns` - The amount of nanoseconds to wait.
pub fn wait(ns: u64) {
    let start = read_tsc();
    let delta = ns * CLOCK_CYCLES_PER_NS.load(Ordering::Relaxed);

    while read_tsc() - start < delta {
        spin_loop();
    }
}

/// Sets the PIT frequency divider.
///
/// # Arguments
///
/// * `divider` - The PIT frequency divider.
/// * `channel` - The PIT channel.
///
/// # Returns
///
/// * `Result<(), Error>` - The result of the operation.
///
/// # Errors
///
/// * If the PIT frequency divider is invalid.
/// * If PIT command byte conversion fails.
pub fn set_pit_frequency_divider(divider: u16, channel: &Channel) -> Result<(), Error> {
    // Converts the channel to a u16 and gets the access mode and operation mode.
    let channel = u16::from(*channel);
    let access_mode = u16::from(AccessMode::LowByteThenHighByte);
    let operation_mode = u16::from(OperatingMode::HardwareTriggeredStrobe);

    interrupts::without_interrupts(|| {
        let bytes = divider.to_le_bytes();

        // Checks if the PIT frequency divider is valid, meaning the frequency is not zero.
        if bytes[0] == 0 && bytes[1] == 0 {
            return Err(Error::Internal(
                "The PIT frequency divider cannot be zero!".into(),
            ));
        }

        let mut command: Port<u8> = Port::new(0x43); // The PIT command port.
        let mut data: Port<u8> = Port::new(channel); // The PIT data port.

        // Writes the PIT frequency divider to the PIT.
        unsafe {
            command.write((channel << 6 | access_mode << 4 | operation_mode) as u8);

            data.write(bytes[0]);
            data.write(bytes[1]);
        }

        Ok(())
    })
}
