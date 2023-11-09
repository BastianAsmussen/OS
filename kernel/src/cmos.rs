use core::hint::spin_loop;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::instructions::port::Port;

/// Where the CMOS address is located.
const CMOS_ADDRESS: u8 = 0x70;
/// Where the CMOS data is located.
const CMOS_DATA: u8 = 0x71;

/// The CMOS registers.
///
/// # Variants
///
/// * [`Register::Seconds`]
/// * [`Register::Minutes`]
/// * [`Register::Hours`]
/// * [`Register::Day`]
/// * [`Register::Month`]
/// * [`Register::Year`]
/// * [`Register::Century`]
/// * [`Register::StatusA`]
/// * [`Register::StatusB`]
///
/// # See
///
/// * [CMOS](https://wiki.osdev.org/CMOS)
#[derive(Clone, Copy)]
pub enum Register {
    /// The seconds register, which is located at `0x00`.
    Seconds = 0x00,
    /// The minutes register, which is located at `0x02`.
    Minutes = 0x02,
    /// The hours register, which is located at `0x04`.
    Hours = 0x04,
    /// The day register, which is located at `0x07`.
    Day = 0x07,
    /// The month register, which is located at `0x08`.
    Month = 0x08,
    /// The year register, which is located at `0x09`.
    Year = 0x09,
    /// The century register, which is located at `0x32`.
    Century = 0x32,
    /// The status A register, which is located at `0x0A`.
    ///
    /// # Notes
    ///
    /// * This register is used for storing information about RTC updates:
    ///   * `Bit 7` - RTC update in progress. (0 = No, 1 = Yes)
    StatusA = 0x0A,
    /// The status B register, which is located at `0x0B`.
    ///
    /// # Notes
    ///
    /// * This register is used for storing information about the RTC:
    ///   * `Bit 1` - Enable/disable 24-hour format. (0 = 12-hour, 1 = 24-hour)
    ///   * `Bit 2` - Enable/disable binary mode. (0 = BCD, 1 = Binary)
    StatusB = 0x0B,
    StatusC = 0x0C,
}

impl From<u8> for Register {
    /// Converts a `u8` to a `Register`.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to convert.
    ///
    /// # Returns
    ///
    /// * `Register` - The converted value. If the value is not a valid register, then [`Register::Seconds`] is returned.
    fn from(value: u8) -> Self {
        match value {
            0x02 => Self::Minutes,
            0x04 => Self::Hours,
            0x07 => Self::Day,
            0x08 => Self::Month,
            0x09 => Self::Year,
            0x32 => Self::Century,
            0x0A => Self::StatusA,
            0x0B => Self::StatusB,
            _ => Self::Seconds,
        }
    }
}

/// The CMOS.
///
/// # Fields
///
/// * `addr` - The CMOS address port.
/// * `data` - The CMOS data port.
#[derive(Debug)]
pub struct CMOS {
    addr: Port<u8>,
    data: Port<u8>,
}

impl CMOS {
    /// Creates a new `CMOS` instance with the default CMOS address and data ports.
    ///
    /// # See
    ///
    /// * [CMOS](https://wiki.osdev.org/CMOS)
    /// * [`CMOS_ADDRESS`]
    /// * [`CMOS_DATA`]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            addr: Port::new(CMOS_ADDRESS as u16),
            data: Port::new(CMOS_DATA as u16),
        }
    }

    /// Reads from the given register.
    ///
    /// # Arguments
    ///
    /// * `reg` - The register to read from.
    ///
    /// # Returns
    ///
    /// * `u8` - The value of the register.
    pub fn read(&mut self, reg: &Register) -> u8 {
        unsafe {
            self.addr.write(*reg as u8);
            self.data.read()
        }
    }

    /// Writes to the given register.
    ///
    /// # Arguments
    ///
    /// * `reg` - The register to write to.
    /// * `value` - The value to write.
    pub fn write(&mut self, reg: &Register, value: u8) {
        unsafe {
            self.addr.write(*reg as u8);
            self.data.write(value);
        }
    }

    /// Reads from the previous register.
    ///
    /// # Returns
    ///
    /// * `Register` - The previous register.
    pub fn prev_addr(&mut self) -> Register {
        let prev = unsafe { self.addr.read() };

        Register::from(prev)
    }

    /// Reads from the previous data.
    ///
    /// # Returns
    ///
    /// * `u8` - The previous data.
    pub fn prev_data(&mut self) -> u8 {
        unsafe { self.data.read() }
    }

    /// Sets whether or not the NMI is enabled or not for the previous register.
    ///
    /// # Arguments
    ///
    /// * `reg` - The register to set the NMI for.
    /// * `enabled` - True if the NMI should be enabled, false if it should be disabled.
    pub fn set_nmi(&mut self, reg: &Register, enabled: bool) {
        let nmi_bit = 1 << 7;
        let value = if enabled {
            *reg as u8 | nmi_bit
        } else {
            *reg as u8 & !nmi_bit
        };

        self.write(reg, value);
    }

    /// Gets whether or not the NMI is disabled or enabled for the given register.
    ///
    /// # Arguments
    ///
    /// * `reg` - The register to check.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether or not the NMI is enabled or not.
    pub fn is_nmi_disabled(&mut self, reg: &Register) -> bool {
        let value = self.read(reg);
        let nmi_bit = 1 << 7;

        // If the NMI bit is 0, then the NMI is enabled, and vice versa.
        value & nmi_bit == 0
    }
}

impl Default for CMOS {
    /// Creates a new `CMOS` instance with the default CMOS address and data ports.
    ///
    /// # Returns
    ///
    /// * `CMOS` - The new instance.
    fn default() -> Self {
        Self::new()
    }
}

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
        if !self.is_binary_mode() {
            seconds = Self::bcd_to_binary(seconds);
            minutes = Self::bcd_to_binary(minutes);
            hours = Self::bcd_to_binary(hours);
            day = Self::bcd_to_binary(day);
            month = Self::bcd_to_binary(month);
            year = Self::bcd_to_binary(year);
            century = Self::bcd_to_binary(century);
        }

        // If the RTC is in 12-hour mode, then convert the hours to 24-hour mode.
        if !self.is_24_hour_mode() {
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
    pub fn is_rtc_updating(&mut self) -> bool {
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
        while self.is_rtc_updating() {
            spin_loop();
        }
    }

    /// Gets whether or not the RTC is in 24-hour mode.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether or not the RTC is in 24-hour mode or not.
    fn is_24_hour_mode(&mut self) -> bool {
        let value = self.cmos.read(&Register::StatusB);

        // If the first bit is 0, then the RTC is in 12-hour mode, and vice versa.
        value & 1 == 0
    }

    /// Gets whether or not the RTC is in binary mode.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether or not the RTC is in binary mode or in BCD mode.
    fn is_binary_mode(&mut self) -> bool {
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
