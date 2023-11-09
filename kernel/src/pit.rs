/// The PIT channels.
///
/// # Variants
///
/// * `Channel0` - Channel 0, used for generating interrupts.
/// * `Channel1` - Channel 1, used for reading the clock.
/// * `Channel2` - Channel 2, used for the PC speaker.
///
/// # See
///
/// * [PIT](https://wiki.osdev.org/Programmable_Interval_Timer)
pub enum Channel {
    Channel0 = 0x40,
    Channel1 = 0x41,
    Channel2 = 0x42,
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
/// * `LoByteOnly` - Lo byte only command.
/// * `HiByteOnly` - Hi byte only command.
/// * `LoByteThenHiByte` - Lo byte then hi byte command.
pub enum AccessMode {
    LatchCountValue = 0,
    LoByteOnly = 1,
    HiByteOnly = 2,
    LoByteThenHiByte = 3,
}

impl From<AccessMode> for u8 {
    /// Converts a `PitAccessMode` to a `u8`.
    ///
    /// # Arguments
    ///
    /// * `access_mode` - The access mode to convert.
    ///
    /// # Returns
    ///
    /// * `u8` - The converted access mode.
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

impl From<OperatingMode> for u8 {
    /// Converts a `PitOperatingMode` to a `u8`.
    ///
    /// # Arguments
    ///
    /// * `operating_mode` - The operating mode to convert.
    ///
    /// # Returns
    ///
    /// * `u8` - The converted operating mode.
    fn from(operating_mode: OperatingMode) -> Self {
        operating_mode as Self
    }
}
