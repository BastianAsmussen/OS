use crate::dev::ata::registers::Register;
use crate::errors::Error;
use crate::sys::time;
use crate::sys::time::clock;
use bit_field::BitField;
use core::hint::spin_loop;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

/// The different possible device types.
///
/// # Variants
///
/// * `ATA` - An ATA device.
/// * `ATAPI` - An ATAPI device.
/// * `SATA` - A SATA device.
/// * `SATAPI` - A SATAPI device.
/// * `Unknown` - An unknown device.
#[derive(Debug)]
pub enum DeviceType {
    ATA,
    ATAPI,
    SATA,
    SATAPI,
    Unknown,
}

/// The different possible status flags.
///
/// # Variants
///
/// * `Error` - Indicates an error occurred. Send a new command to clear it (or nuke it with a Software Reset).
/// * `Index` - Index. Always set to zero.
/// * `CorrectedData` - Corrected data. Always set to zero.
/// * `DataRequest` - Set when the drive has PIO data to transfer, or is ready to accept PIO data.
/// * `ServiceRequest` - Overlapped Mode Service Request.
/// * `DriveFault` - Drive Fault Error (does not set ERR).
/// * `DriveReady` - Bit is clear when drive is spun down, or after an error. Set otherwise.
/// * `Busy` - Indicates the drive is preparing to send/receive data (wait for it to clear). In case of 'hang' (it never clears), do a software reset.
#[derive(Debug, Clone, Copy)]
pub enum Status {
    Error = 0,
    Index = 1,
    CorrectedData = 2,
    DataRequest = 3,
    ServiceRequest = 4,
    DriveFault = 5,
    DriveReady = 6,
    Busy = 7,
}

/// The different possible commands.
///
/// # Variants
///
/// * `Identify` - Identify the device.
/// * `Read` - Read from the device.
/// * `Write` - Write to the device.
#[derive(Debug, Clone, Copy)]
pub enum Command {
    Identify = 0xEC,
    Read = 0x20,
    Write = 0x30,
}

/// An ATA bus, used to communicate with ATA devices.
///
/// # Fields
///
/// * `id` - The ID of the bus.
/// * `irq` - The IRQ of the bus.
///
/// ## Registers
///
/// * `status_register` - The status register.
/// * `alternate_status_register` - The alternate status register.
/// * `error_register` - The error register.
/// * `device_block_register` - The device block register.
/// * `command_register` - The command register.
/// * `control_register` - The control register.
/// * `features_register` - The features register.
/// * `data_register` - The data register.
/// * `drive_register` - The drive register.
/// * `sector_count_register` - The sector count register.
///
/// ## LBA Registers
///
/// * `lba0_register` - The LBA0 register.
/// * `lba1_register` - The LBA1 register.
/// * `lba2_register` - The LBA2 register.
#[derive(Debug)]
pub struct Bus {
    id: u8,
    irq: u8,

    status_register: PortReadOnly<u8>,
    alternate_status_register: PortReadOnly<u8>,
    error_register: PortReadOnly<u8>,
    device_block_register: PortReadOnly<u8>,
    command_register: PortWriteOnly<u8>,
    control_register: PortWriteOnly<u8>,
    features_register: PortWriteOnly<u8>,
    data_register: Port<u16>,
    drive_register: Port<u8>,
    sector_count_register: Port<u8>,

    lba0_register: Port<u8>,
    lba1_register: Port<u8>,
    lba2_register: Port<u8>,
}

impl Bus {
    /// Creates a new bus.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the bus.
    /// * `irq` - The IRQ of the bus.
    /// * `io_base` - The I/O base of the bus.
    /// * `control_base` - The control base of the bus.
    #[must_use]
    pub const fn new(id: u8, irq: u8, io_base: u16, control_base: u16) -> Self {
        Self {
            id,
            irq,

            status_register: PortReadOnly::new(Register::Status(io_base).offset()),
            alternate_status_register: PortReadOnly::new(
                Register::AlternateStatus(control_base).offset(),
            ),
            error_register: PortReadOnly::new(Register::Error(io_base).offset()),
            device_block_register: PortReadOnly::new(Register::DriveHead(io_base).offset()),
            command_register: PortWriteOnly::new(Register::Command(io_base).offset()),
            control_register: PortWriteOnly::new(Register::DeviceControl(control_base).offset()),
            features_register: PortWriteOnly::new(Register::Features(io_base).offset()),
            data_register: Port::new(Register::Data(io_base).offset()),
            drive_register: Port::new(Register::DriveHead(io_base).offset()),
            sector_count_register: Port::new(Register::SectorCount(io_base).offset()),

            lba0_register: Port::new(Register::SectorNumber(io_base).offset()),
            lba1_register: Port::new(Register::CylinderLow(io_base).offset()),
            lba2_register: Port::new(Register::CylinderHigh(io_base).offset()),
        }
    }

    /// Reads the value of a register.
    ///
    /// # Arguments
    ///
    /// * `register` - The register to read.
    ///
    /// # Returns
    ///
    /// * `Result<u8, Error>` - The value of the register.
    ///
    /// # Errors
    ///
    /// * `Error::Internal` - The register is not readable.
    pub fn read_register(&mut self, register: &Register) -> Result<u8, Error> {
        let value = unsafe {
            match register {
                Register::Data(_) => u8::try_from(self.data_register.read())?,
                Register::Error(_) => self.error_register.read(),
                Register::SectorCount(_)
                | Register::SectorNumber(_)
                | Register::CylinderLow(_)
                | Register::CylinderHigh(_) => self.sector_count_register.read(),
                Register::DriveHead(_) => self.drive_register.read(),
                Register::Status(_) => self.status_register.read(),
                Register::AlternateStatus(_) => self.alternate_status_register.read(),
                Register::DeviceControl(_) => self.device_block_register.read(),

                // Write-only registers.
                _ => return Err(Error::Internal("Not a readable register!".into())),
            }
        };

        Ok(value)
    }

    /// Writes a value to a register.
    ///
    /// # Arguments
    ///
    /// * `register` - The register to write to.
    /// * `value` - The value to write.
    ///
    /// # Errors
    ///
    /// * `Error::Internal` - The register is not writable.
    pub fn write_register(&mut self, register: &Register, value: u8) -> Result<(), Error> {
        unsafe {
            match register {
                Register::Data(_) => self.data_register.write(u16::from(value)),
                Register::SectorCount(_)
                | Register::SectorNumber(_)
                | Register::CylinderLow(_)
                | Register::CylinderHigh(_) => self.sector_count_register.write(value),
                Register::DriveHead(_) | Register::DriveAddress(_) => {
                    self.drive_register.write(value);
                }
                Register::Command(_) => self.command_register.write(value),
                Register::Features(_) => self.features_register.write(value),
                Register::DeviceControl(_) => self.control_register.write(value),

                // Read-only registers.
                _ => return Err(Error::Internal("Not a writable register!".into())),
            }
        };

        Ok(())
    }

    /// Reads the status of the bus.
    ///
    /// # Returns
    ///
    /// * `Result<u8, Error>` - The status of the bus.
    ///
    /// # Errors
    ///
    /// * `Error::Internal` - The status register is not readable.
    pub fn status(&mut self) -> Result<u8, Error> {
        self.read_register(&Register::AlternateStatus(0))
    }

    /// Clears the interrupt.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the clearing.
    ///
    /// # Errors
    ///
    /// * If the status register is not readable.
    pub fn clear_interrupt(&mut self) -> Result<(), Error> {
        self.read_register(&Register::Status(0))?;

        Ok(())
    }

    /// Checks if the bus is floating.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Error>` - Whether the bus is floating.
    ///
    /// # Errors
    ///
    /// * `Error::Internal` - The status register is not readable.
    pub fn floating_bus(&mut self) -> Result<bool, Error> {
        Ok(matches!(self.status()?, 0xFF | 0x7F))
    }

    /// Reads the error of the bus.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Error>` - If an error occurred.
    ///
    /// # Errors
    ///
    /// * `Error::Internal` - The status register is not readable.
    pub fn error(&mut self) -> Result<bool, Error> {
        Ok(self.status()?.get_bit(Status::Error as usize))
    }

    /// Polls a bit in the status register.
    ///
    /// # Arguments
    ///
    /// * `bit` - The bit to poll.
    /// * `value` - The value to poll for.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the polling.
    ///
    /// # Errors
    ///
    /// * If the polling times out.
    pub fn poll(&mut self, bit: Status, value: bool) -> Result<(), Error> {
        let start_time = clock::uptime();
        while self.status()?.get_bit(bit as usize) != value {
            if clock::uptime() - start_time > 1.0 {
                return Err(Error::Internal("Timed out while polling!".into()));
            }

            spin_loop();
        }

        Ok(())
    }

    /// Selects a device.
    ///
    /// # Arguments
    ///
    /// * `device` - The device to select.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the selection.
    ///
    /// # Errors
    ///
    /// * If the device is not a valid device.
    /// * If the device fails to poll.
    pub fn select_drive(&mut self, drive: u8) -> Result<(), Error> {
        self.poll(Status::Busy, false)?;
        self.poll(Status::DataRequest, false)?;

        self.write_register(&Register::DriveHead(0), 0xA0 | (drive << 4))?;
        time::wait(400); // Wait 400 ns for the drive to select.

        self.poll(Status::Busy, false)?;
        self.poll(Status::DataRequest, false)?;

        Ok(())
    }
}
