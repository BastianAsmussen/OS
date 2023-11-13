/// Direction of data transfer.
///
/// # Variants
///
/// * `Read` - Read from the device.
/// * `Write` - Write to the device.
/// * `Both` - Read and write to the device.
#[derive(Debug)]
pub enum Direction {
    Read,
    Write,
    Both,
}

/// Registers for ATA devices.
///
/// # Variants
///
/// * `IO(IORegister)` - I/O registers.
/// * `Control(ControlRegister)` - Control registers.
#[derive(Debug)]
pub enum Register {
    IO(IORegister),
    Control(ControlRegister),
}

/// I/O registers for ATA devices.
///
/// # Variants
///
/// * `Data(u16)` - Data register, offset by `0`.
/// * `Error(u16)` - Error register, offset by `1`.
/// * `Features(u16)` - Features register, offset by `1`.
/// * `SectorCount(u16)` - Sector count register, offset by `2`.
/// * `SectorNumber(u16)` - Sector number register, offset by `3`.
/// * `CylinderLow(u16)` - Cylinder low register, offset by `4`.
/// * `CylinderHigh(u16)` - Cylinder high register, offset by `5`.
/// * `DriveHead(u16)` - Drive/head register, offset by `6`.
/// * `Status(u16)` - Status register, offset by `7`.
/// * `Command(u16)` - Command register, offset by `7`.
#[derive(Debug)]
pub enum IORegister {
    Data(u16),
    Error(u16),
    Features(u16),
    SectorCount(u16),
    SectorNumber(u16),
    CylinderLow(u16),
    CylinderHigh(u16),
    DriveHead(u16),
    Status(u16),
    Command(u16),
}

impl IORegister {
    /// Returns the offset of the register.
    #[must_use]
    pub const fn offset(&self) -> u16 {
        match self {
            Self::Data(base) => *base,
            Self::Error(base) | Self::Features(base) => *base + 1,
            Self::SectorCount(base) => *base + 2,
            Self::SectorNumber(base) => *base + 3,
            Self::CylinderLow(base) => *base + 4,
            Self::CylinderHigh(base) => *base + 5,
            Self::DriveHead(base) => *base + 6,
            Self::Status(base) | Self::Command(base) => *base + 7,
        }
    }

    /// Returns the direction of the register.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        match self {
            Self::Data(_)
            | Self::SectorCount(_)
            | Self::SectorNumber(_)
            | Self::CylinderLow(_)
            | Self::CylinderHigh(_)
            | Self::DriveHead(_) => Direction::Both,
            Self::Error(_) | Self::Status(_) => Direction::Read,
            Self::Features(_) | Self::Command(_) => Direction::Write,
        }
    }
}

/// Control registers for ATA devices.
///
/// # Variants
///
/// * `AlternateStatus(u16)` - Alternate status register, offset by `0`.
/// * `DeviceControl(u16)` - Device control register, offset by `0`.
/// * `DriveAddress(u16)` - Drive address register, offset by `1`.
#[derive(Debug)]
pub enum ControlRegister {
    AlternateStatus(u16),
    DeviceControl(u16),
    DriveAddress(u16),
}

impl ControlRegister {
    /// Returns the offset of the register.
    #[must_use]
    pub const fn offset(&self) -> u16 {
        match self {
            Self::AlternateStatus(base) | Self::DeviceControl(base) => *base,
            Self::DriveAddress(base) => *base + 1,
        }
    }

    /// Returns the direction of the register.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        match self {
            Self::AlternateStatus(_) | Self::DriveAddress(_) => Direction::Read,
            Self::DeviceControl(_) => Direction::Write,
        }
    }
}
