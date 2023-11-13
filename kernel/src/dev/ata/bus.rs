use crate::dev::ata::registers::{ControlRegister, IORegister, Register};
use crate::errors::Error;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

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

            status_register: PortReadOnly::new(IORegister::Status(io_base).offset()),
            alternate_status_register: PortReadOnly::new(
                ControlRegister::AlternateStatus(control_base).offset(),
            ),
            error_register: PortReadOnly::new(IORegister::Error(io_base).offset()),
            device_block_register: PortReadOnly::new(IORegister::DriveHead(io_base).offset()),
            command_register: PortWriteOnly::new(IORegister::Command(io_base).offset()),
            control_register: PortWriteOnly::new(
                ControlRegister::DeviceControl(control_base).offset(),
            ),
            features_register: PortWriteOnly::new(IORegister::Features(io_base).offset()),
            data_register: Port::new(IORegister::Data(io_base).offset()),
            drive_register: Port::new(IORegister::DriveHead(io_base).offset()),
            sector_count_register: Port::new(IORegister::SectorCount(io_base).offset()),
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
                Register::Control(control_register) => match control_register {
                    ControlRegister::AlternateStatus(_) => self.alternate_status_register.read(),
                    ControlRegister::DeviceControl(_) => self.device_block_register.read(),
                    ControlRegister::DriveAddress(_) => {
                        return Err(Error::Internal("Not a readable register!".into()))
                    }
                },
                Register::IO(io_register) => match io_register {
                    IORegister::Data(_) => u8::try_from(self.data_register.read())?,
                    IORegister::Error(_) => self.error_register.read(),
                    IORegister::SectorCount(_)
                    | IORegister::SectorNumber(_)
                    | IORegister::CylinderLow(_)
                    | IORegister::CylinderHigh(_) => self.sector_count_register.read(),
                    IORegister::DriveHead(_) => self.drive_register.read(),
                    IORegister::Status(_) => self.status_register.read(),
                    _ => return Err(Error::Internal("Not a readable register!".into())),
                },
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
                Register::Control(control_register) => match control_register {
                    ControlRegister::DeviceControl(_) => self.control_register.write(value),
                    ControlRegister::DriveAddress(_) => self.drive_register.write(value),
                    ControlRegister::AlternateStatus(_) => {
                        return Err(Error::Internal("Not a writable register!".into()))
                    }
                },
                Register::IO(io_register) => match io_register {
                    IORegister::Data(_) => self.data_register.write(u16::from(value)),
                    IORegister::SectorCount(_)
                    | IORegister::SectorNumber(_)
                    | IORegister::CylinderLow(_)
                    | IORegister::CylinderHigh(_) => self.sector_count_register.write(value),
                    IORegister::DriveHead(_) => self.drive_register.write(value),
                    IORegister::Command(_) => self.command_register.write(value),
                    IORegister::Features(_) => self.features_register.write(value),
                    _ => return Err(Error::Internal("Not a writable register!".into())),
                },
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
        self.read_register(&Register::Control(ControlRegister::AlternateStatus(0)))
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
}
