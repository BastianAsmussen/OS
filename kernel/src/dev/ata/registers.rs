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
/// * `Data(u16)` - Data register, offset from I/O base by `0`.
/// * `Error(u16)` - Error register, offset from I/O base by `1`.
/// * `Features(u16)` - Features register, offset from I/O base by `1`.
/// * `SectorCount(u16)` - Sector count register, offset from I/O base by `2`.
/// * `SectorNumber(u16)` - Sector number register, offset from I/O base by `3`.
/// * `CylinderLow(u16)` - Cylinder low register, offset from I/O base by `4`.
/// * `CylinderHigh(u16)` - Cylinder high register, offset from I/O base by `5`.
/// * `DriveHead(u16)` - Drive/head register, offset from I/O base by `6`.
/// * `Status(u16)` - Status register, offset from I/O base by `7`.
/// * `Command(u16)` - Command register, offset from I/O base by `7`.
/// * `Control(ControlRegister)` - Control registers.
///
/// * `AlternateStatus(u16)` - Alternate status register, offset from control base by `0`.
/// * `DeviceControl(u16)` - Device control register, offset from control base by `0`.
/// * `DriveAddress(u16)` - Drive address register, offset from control base by `1`.
#[derive(Debug)]
pub enum Register {
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

    AlternateStatus(u16),
    DeviceControl(u16),
    DriveAddress(u16),
}

impl Register {
    /// Returns the offset of the register.
    #[must_use]
    pub const fn offset(&self) -> u16 {
        match self {
            Self::AlternateStatus(base) | Self::DeviceControl(base) | Self::Data(base) => *base,
            Self::Error(base) | Self::Features(base) | Self::DriveAddress(base) => *base + 1,
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
            Self::Error(_) | Self::Status(_) | Self::AlternateStatus(_) | Self::DriveAddress(_) => {
                Direction::Read
            }
            Self::Features(_) | Self::Command(_) | Self::DeviceControl(_) => Direction::Write,
        }
    }
}
