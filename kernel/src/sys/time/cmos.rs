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
    pub fn nmi_disabled(&mut self, reg: &Register) -> bool {
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
