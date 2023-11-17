use crate::errors::Error;
use crate::sys::time::wait;
use crate::{errors, println};
use alloc::format;
use bit_field::BitField;
use x86_64::instructions::port::Port;

/// The ATA register.
///
/// # Variants
///
/// * `Data` - The data register.
/// * `Error` - The error register.
/// * `Features` - The features register.
/// * `SectorCount` - The sector count register.
/// * `SectorNumber` - The sector number register.
/// * `CylinderLow` - The cylinder low register.
/// * `CylinderHigh` - The cylinder high register.
/// * `DriveHead` - The drive/head register.
/// * `Status` - The status register.
/// * `Command` - The command register.
/// * `AlternateStatus` - The alternate status register.
/// * `DeviceControl` - The device control register.
/// * `DriveAddress` - The drive address register.
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
    /// Gets the offset for the register.
    ///
    /// # Returns
    ///
    /// * `u16` - The offset for the register.
    #[must_use]
    pub const fn offset(&self) -> u16 {
        match self {
            Self::Data(io_base) => *io_base,
            Self::Error(io_base) | Self::Features(io_base) => *io_base + 1,
            Self::SectorCount(io_base) => *io_base + 2,
            Self::SectorNumber(io_base) => *io_base + 3,
            Self::CylinderLow(io_base) => *io_base + 4,
            Self::CylinderHigh(io_base) => *io_base + 5,
            Self::DriveHead(io_base) => *io_base + 6,
            Self::Status(io_base) | Self::Command(io_base) => *io_base + 7,
            Self::AlternateStatus(control_base) | Self::DeviceControl(control_base) => {
                *control_base
            }
            Self::DriveAddress(control_base) => *control_base + 1,
        }
    }

    /// Gets the direction of the register.
    ///
    /// # Returns
    ///
    /// * `Direction` - The direction of the register.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        match self {
            Self::Data(_)
            | Self::SectorCount(_)
            | Self::SectorNumber(_)
            | Self::CylinderLow(_)
            | Self::CylinderHigh(_)
            | Self::DriveHead(_) => Direction::Both,
            Self::Error(_) | Self::DriveAddress(_) | Self::Status(_) | Self::AlternateStatus(_) => {
                Direction::Read
            }
            Self::Features(_) | Self::Command(_) | Self::DeviceControl(_) => Direction::Write,
        }
    }

    /// Reads from the given register.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus to read from.
    ///
    /// # Returns
    ///
    /// * `Result<u8, Error>` - The value of the register.
    ///
    /// # Errors
    ///
    /// * If the register is not readable.
    pub fn read(&self) -> Result<u8, Error> {
        // Check if the register is readable.
        match self.direction() {
            Direction::Read | Direction::Both => {}
            Direction::Write => {
                return Err(Error::InvalidRegister(format!(
                    "Register {self:#?} is not readable!"
                )))
            }
        }

        let port_number = self.offset();
        let value = unsafe {
            let mut port = Port::new(port_number);

            port.read()
        };

        Ok(value)
    }

    /// Writes to the given register.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus to write to.
    /// * `value` - The value to write to the register.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the register is not writable.
    pub fn write(&self, value: u8) -> Result<(), errors::Error> {
        // Check if the register is writable.
        match self.direction() {
            Direction::Write | Direction::Both => {}
            Direction::Read => {
                return Err(Error::InvalidRegister(format!(
                    "Register {self:#?} is not writable!"
                )))
            }
        }

        let port_number = self.offset();
        unsafe {
            let mut port = Port::new(port_number);

            port.write(value);
        }

        Ok(())
    }
}

/// The direction of the register.
///
/// # Variants
///
/// * `Read` - The register is readable.
/// * `Write` - The register is writable.
/// * `Both` - The register is readable and writable.
#[derive(Debug)]
pub enum Direction {
    Read,
    Write,
    Both,
}

/// The register handler
///
/// # Fields
///
/// * `io_base` - The I/O base port.
/// * `control_base` - The control base port.
#[derive(Debug)]
pub struct RegisterHandler {
    pub io_base: u16,
    pub control_base: u16,
}

impl RegisterHandler {
    /// Creates a new register handler.
    ///
    /// # Arguments
    ///
    /// * `io_base` - The I/O base port.
    /// * `control_base` - The control base port.
    ///
    /// # Returns
    ///
    /// * `Self` - The register handler.
    #[must_use]
    pub const fn new(io_base: u16, control_base: u16) -> Self {
        Self {
            io_base,
            control_base,
        }
    }

    /// Gets the status of the drive.
    ///
    /// # Returns
    ///
    /// * `Result<u8, Error>` - The status of the drive.
    ///
    /// # Errors
    ///
    /// * If the status register is not readable.
    pub fn status(&self) -> Result<u8, Error> {
        let register = Register::AlternateStatus(self.control_base);

        // We need to read the alternate status register 16 times total to ensure that the drive is ready.
        for _ in 0..15 {
            register.read()?;
            wait(400);
        }

        // Read the status register, and return it.
        let status = register.read()?;

        Ok(status)
    }

    /// Gets the error, if any.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ErrorKind>, Error>` - The error, if any.
    ///
    /// # Errors
    ///
    /// * If the error register is not readable.
    /// * If the error kind is unknown.
    pub fn error(&self) -> Result<Option<ErrorKind>, Error> {
        let status = self.status()?;

        // Firstly, check if the error bit is set, if it isn't, then there is no error.
        if !status.get_bit(6) {
            return Ok(None);
        }

        // If it is, then check which error it is by reading the error register.
        let value = Register::Error(self.io_base).read()?;

        let error_kind = ErrorKind::try_from(value)?;

        Ok(Some(error_kind))
    }
}

/// The error register.
///
/// # Variants
///
/// * `AddressMarkNotFound` - Indicates an error occurred. Send a new command to clear it (or nuke it with a Software Reset).
/// * `TrackZeroNotFound` - Index. Always set to zero.
/// * `AbortedCommand` - Corrected data. Always set to zero.
/// * `MediaChangeRequest` - Set when the drive has PIO data to transfer, or is ready to accept PIO data.
/// * `IdNotFound` - Overlapped Mode Service Request.
/// * `MediaChanged` - Drive Fault Error (does not set ERR).
/// * `UncorrectableDataError` - Bit is clear when drive is spun down, or after an error. Set otherwise.
/// * `BadBlockDetected` - Indicates the drive is preparing to send/receive data (wait for it to clear). In case of 'hang' (it never clears), do a software reset.
#[derive(Debug)]
pub enum ErrorKind {
    AddressMarkNotFound,
    TrackZeroNotFound,
    AbortedCommand,
    MediaChangeRequest,
    IdNotFound,
    MediaChanged,
    UncorrectableDataError,
    BadBlockDetected,
}

impl TryFrom<u8> for ErrorKind {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        /*
        Bit	Abbreviation	Function
        0	AMNF	Address mark not found.
        1	TKZNF	Track zero not found.
        2	ABRT	Aborted command.
        3	MCR	Media change request.
        4	IDNF	ID not found.
        5	MC	Media changed.
        6	UNC	Uncorrectable data error.
        7	BBK	Bad Block detected.
         */

        let error_kind = match value {
            0x01 => Self::AddressMarkNotFound,
            0x02 => Self::TrackZeroNotFound,
            0x04 => Self::AbortedCommand,
            0x08 => Self::MediaChangeRequest,
            0x10 => Self::IdNotFound,
            0x20 => Self::MediaChanged,
            0x40 => Self::UncorrectableDataError,
            0x80 => Self::BadBlockDetected,
            _ => return Err(Error::Conversion(format!("Unknown Error Kind: {value:#X}"))),
        };

        Ok(error_kind)
    }
}
