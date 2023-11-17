use crate::sys::time::cmos::{Register, CMOS};
use x86_64::instructions::interrupts::without_interrupts;

/// The real time clock.
///
/// # Fields
///
/// * `cmos` - The CMOS.
///
/// * `seconds` - The seconds.
/// * `minutes` - The minutes.
/// * `hours` - The hours.
/// * `day` - The day.
/// * `month` - The month.
/// * `year` - The year.
/// * `century` - The century.
#[derive(Debug, Default)]
pub struct RTC {
    cmos: CMOS,

    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub day: u8,
    pub month: u8,
    pub year: u8,
    pub century: u8,
}

impl RTC {
    /// Creates a new `RTC` instance and updates it.
    ///
    /// # Notes
    ///
    /// * This function will wait for the RTC to finish updating.
    #[must_use]
    pub fn new() -> Self {
        let mut rtc = Self::default();

        rtc.wait_for_rtc_update();
        rtc.update();
        rtc
    }

    /// Creates a new `RTC` instance without checking if the RTC is updating.
    #[must_use]
    pub fn new_no_check() -> Self {
        let mut rtc = Self::default();

        rtc.update();
        rtc
    }

    /// Updates the RTC.
    ///
    /// # Notes
    ///
    /// * This function won't wait for the RTC to finish updating.
    pub fn update(&mut self) {
        let mut seconds = self.cmos.read(&Register::Seconds);
        let mut minutes = self.cmos.read(&Register::Minutes);
        let mut hours = self.cmos.read(&Register::Hours);
        let mut day = self.cmos.read(&Register::Day);
        let mut month = self.cmos.read(&Register::Month);
        let mut year = self.cmos.read(&Register::Year);
        let mut century = self.cmos.read(&Register::Century);

        // If the RTC is in BCD mode, then convert the values to binary.
        if !self.binary_mode() {
            seconds = Self::bcd_to_binary(seconds);
            minutes = Self::bcd_to_binary(minutes);
            hours = Self::bcd_to_binary(hours);
            day = Self::bcd_to_binary(day);
            month = Self::bcd_to_binary(month);
            year = Self::bcd_to_binary(year);
            century = Self::bcd_to_binary(century);
        }

        // If the RTC is in 12-hour mode, then convert the hours to 24-hour mode.
        if !self.military_time_mode() {
            // If the PM bit is set, then add 12 to the hours.
            if hours & 0x80 != 0 {
                hours = (hours & 0x7F) + 12;
            }
        }

        self.seconds = seconds;
        self.minutes = minutes;
        self.hours = hours;
        self.day = day;
        self.month = month;
        self.year = year;
        self.century = century;
    }

    /// Gets whether or not the RTC is updating.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether or not the RTC is updating.
    pub fn rtc_updating(&mut self) -> bool {
        let status = self.cmos.read(&Register::StatusA);
        let update_bit = 1 << 7;

        // If the RTC update in progress bit is 0, then the RTC is not updating, and vice versa.
        status & update_bit == 0
    }

    /// Waits for the RTC to finish updating.
    ///
    /// # Notes
    ///
    /// * This function will spin until the RTC is done updating.
    pub fn wait_for_rtc_update(&mut self) {
        while self.rtc_updating() {
            core::hint::spin_loop();
        }
    }

    /// Gets whether or not the RTC is in 24-hour mode, or in 12-hour mode.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether or not the RTC is in 24-hour mode or not.
    fn military_time_mode(&mut self) -> bool {
        let value = self.cmos.read(&Register::StatusB);

        // If the first bit is 0, then the RTC is in 12-hour mode, and vice versa.
        value & 1 == 0
    }

    /// Gets whether or not the RTC is in binary mode.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether or not the RTC is in binary mode or in BCD mode.
    fn binary_mode(&mut self) -> bool {
        let value = self.cmos.read(&Register::StatusB);

        // If the second bit is 0, then the RTC is in BCD mode, and vice versa.
        value & 2 == 0
    }

    /// Disables the given interrupt.
    ///
    /// # Arguments
    ///
    /// * `interrupt` - The interrupt to disable.
    /// * `enabled` - Whether or not the interrupt should be enabled.
    pub fn set_interrupt(&mut self, interrupt: &RTCInterrupt, enabled: bool) {
        without_interrupts(|| {
            // Get the previous register.
            let prev_addr = self.cmos.prev_addr();
            // Disable NMI to prevent the RTC from updating.
            self.cmos.set_nmi(&prev_addr, false);

            // Get the previous data.
            let prev_data = self.cmos.read(&Register::StatusB);
            let value = if enabled {
                prev_data | *interrupt as u8 // Enable the interrupt.
            } else {
                prev_data & !(*interrupt as u8) // Disable the interrupt.
            };
            self.cmos.write(&Register::StatusB, value);

            // Re-enable NMI to allow the RTC to update.
            self.cmos.set_nmi(&prev_addr, true);

            self.notify_interrupt_end();
        });
    }

    /// Sets the rate of the periodic interrupt.
    ///
    /// # Arguments
    ///
    /// * `rate` - The rate of the periodic interrupt.
    ///
    /// # Notes
    ///
    /// * This won't enable the periodic interrupt if it's disabled.
    pub fn set_periodic_rate(&mut self, rate: u8) {
        without_interrupts(|| {
            // Get the previous register.
            let prev_addr = self.cmos.prev_addr();
            // Disable NMI to prevent the RTC from updating.
            self.cmos.set_nmi(&prev_addr, false);

            // Set the rate of the periodic interrupt to the provided rate.
            let prev_data = self.cmos.read(&Register::StatusA);
            let value = (prev_data & 0xF0) | rate;
            self.cmos.write(&Register::StatusA, value);

            // Re-enable NMI to allow the RTC to update.
            self.cmos.set_nmi(&prev_addr, true);

            self.notify_interrupt_end();
        });
    }

    /// Notifies the RTC that the interrupt has ended.
    pub fn notify_interrupt_end(&mut self) {
        self.cmos.read(&Register::StatusC);
    }

    /// Converts the given BCD value to a binary value.
    ///
    /// # Arguments
    ///
    /// * `value` - The BCD value to convert.
    ///
    /// # Returns
    ///
    /// * `u8` - The binary value.
    ///
    /// # Notes
    ///
    /// * This function is used for converting the RTC values from BCD to binary.
    /// * The formula for converting BCD to binary is: `((bcd & 0xF0) >> 1) + ((bcd & 0xF0) >> 3) + (bcd & 0xF)`.
    #[must_use]
    pub const fn bcd_to_binary(value: u8) -> u8 {
        ((value & 0xF0) >> 1) + ((value & 0xF0) >> 3) + (value & 0xF)
    }

    /// Converts the RTC time to milliseconds.
    ///
    /// # Returns
    ///
    /// * `u64` - The RTC time in milliseconds.
    #[must_use]
    pub const fn as_millis(&self) -> u64 {
        let mut millis = 0;

        // Convert the RTC time to milliseconds.
        millis += self.seconds as u64 * 1_000;
        millis += self.minutes as u64 * 60 * 1_000;
        millis += self.hours as u64 * 60 * 60 * 1_000;
        millis += self.day as u64 * 24 * 60 * 60 * 1_000;
        millis += self.month as u64 * 30 * 24 * 60 * 60 * 1_000;
        millis += self.year as u64 * 365 * 24 * 60 * 60 * 1_000;
        millis += self.century as u64 * 100 * 365 * 24 * 60 * 60 * 1_000;

        millis
    }
}

/// The RTC interrupt.
///
/// # Variants
///
/// * [`RTCInterrupt::Periodic`]
/// * [`RTCInterrupt::Alarm`]
/// * [`RTCInterrupt::Update`]
#[derive(Clone, Copy)]
pub enum RTCInterrupt {
    /// The periodic interrupt, which is triggered every 244 microseconds.
    Periodic = 1 << 6,
    /// The alarm interrupt, which is triggered when the RTC alarm goes off.
    Alarm = 1 << 5,
    /// The update interrupt, which is triggered when the RTC updates.
    Update = 1 << 4,
}
