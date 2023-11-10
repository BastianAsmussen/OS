/// The PIT channels.
///
/// # Variants
///
/// * `Zero` - Channel 0, used for generating interrupts.
/// * `One` - Channel 1, used for reading the clock.
/// * `Two` - Channel 2, used for the PC speaker.
///
/// # See
///
/// * [PIT](https://wiki.osdev.org/Programmable_Interval_Timer)
#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Zero = 0x40,
    One = 0x41,
    Two = 0x42,
}

impl From<Channel> for u16 {
    /// Converts a `PitChannel` to a `u16`.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to convert.
    ///
    /// # Returns
    ///
    /// * `u16` - The converted channel.
    fn from(channel: Channel) -> Self {
        channel as Self
    }
}

impl From<Channel> for u8 {
    /// Converts a `PitChannel` to a `u8`.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to convert.
    ///
    /// # Returns
    ///
    /// * `u8` - The converted channel.
    fn from(channel: Channel) -> Self {
        channel as Self
    }
}

/// The PIT access modes.
///
/// # Variants
///
/// * `LatchCountValue` - Latch count value command.
/// * `LowByteOnly` - Low byte only command.
/// * `HighByteOnly` - High byte only command.
/// * `LowByteThenHighByte` - Low byte, then high byte command.
pub enum AccessMode {
    LatchCountValue = 0,
    LowByteOnly = 1,
    HighByteOnly = 2,
    LowByteThenHighByte = 3,
}

impl From<AccessMode> for u16 {
    /// Converts a `PitAccessMode` to a `u16`.
    ///
    /// # Arguments
    ///
    /// * `access_mode` - The access mode to convert.
    ///
    /// # Returns
    ///
    /// * `u16` - The converted access mode.
    fn from(access_mode: AccessMode) -> Self {
        access_mode as Self
    }
}

/// The PIT operating modes.
///
/// # Variants
///
/// * `InterruptOnTerminalCount` - Interrupt on terminal count.
/// * `HardwareRetriggerableOneShot` - Hardware retriggerable one-shot.
/// * `RateGenerator` - Rate generator.
/// * `SquareWaveGenerator` - Square wave generator.
/// * `SoftwareTriggeredStrobe` - Software triggered strobe.
/// * `HardwareTriggeredStrobe` - Hardware triggered strobe.
pub enum OperatingMode {
    InterruptOnTerminalCount = 0,
    HardwareRetriggerableOneShot = 1,
    RateGenerator = 2,
    SquareWaveGenerator = 3,
    SoftwareTriggeredStrobe = 4,
    HardwareTriggeredStrobe = 5,
}

impl From<OperatingMode> for u16 {
    /// Converts a `PitOperatingMode` to a `u16`.
    ///
    /// # Arguments
    ///
    /// * `operating_mode` - The operating mode to convert.
    ///
    /// # Returns
    ///
    /// * `u16` - The converted operating mode.
    fn from(operating_mode: OperatingMode) -> Self {
        operating_mode as Self
    }
}
