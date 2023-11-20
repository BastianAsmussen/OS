use alloc::boxed::Box;
use alloc::{string::String, vec::Vec};
use bit_field::BitField;
use core::{convert::TryInto, hint::spin_loop};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

use crate::errors::Error;
use crate::println;
use crate::sys::time::clock::uptime;
use crate::sys::time::wait;

/// The maximum block size of the ATA bus.
pub const BLOCK_SIZE: usize = 512;

lazy_static! {
    /// The ATA buses.
    pub static ref BUSES: Mutex<Vec<Bus>> = Mutex::new(Vec::new());
}

/// A command.
///
/// # Variants
///
/// * `Identify` - The identify command.
/// * `Read` - The read command.
/// * `Write` - The write command.
#[derive(Debug)]
enum Command {
    Identify = 0xEC,
    Read = 0x20,
    Write = 0x30,
}

/// Represents a device type.
///
/// # Variants
///
/// * `Ata(Box<[u16; 256]>)` - It's an ATA device.
/// * `Atapi` - It's an ATAPI device.
/// * `Sata` - It's a SATA device.
/// * `None` - It's not a recognized device.
#[derive(Debug)]
enum DeviceType {
    Ata(Box<[u16; 256]>),
    Atapi,
    Sata,
    None,
}

/// Represents a status register.
///
/// # Variants
///
/// * `Error` - The error bit.
/// * `Index` - The index bit.
/// * `CorrectedData` - The corrected data bit.
/// * `DataRequest` - The data request bit.
/// * `OverlappedModeServiceRequest` - The overlapped mode service request bit.
/// * `DriveFault` - The drive fault bit.
/// * `Ready` - The ready bit.
/// * `Busy` - The busy bit.
#[derive(Debug, Clone, Copy)]
enum Status {
    Error = 0,
    Index = 1,
    CorrectedData = 2,
    DataRequest = 3,
    OverlappedModeServiceRequest = 4,
    DriveFault = 5,
    Ready = 6,
    Busy = 7,
}

/// Represents a register.
///
/// # Variants
///
/// * `Data(Port<u16>)` - The data register.
/// * `Error(PortReadOnly<u8>)` - The error register.
/// * `Features(PortWriteOnly<u8>)` - The features register.
/// * `SectorCount(Port<u8>)` - The sector count register.
/// * `Lba0(Port<u8>)` - The LBA0 register.
/// * `Lba1(Port<u8>)` - The LBA1 register.
/// * `Lba2(Port<u8>)` - The LBA2 register.
/// * `Drive(Port<u8>)` - The drive register.
/// * `Status(PortReadOnly<u8>)` - The status register.
/// * `Command(PortWriteOnly<u8>)` - The command register.
///
/// * `AlternateStatus(PortReadOnly<u8>)` - The alternate status register.
/// * `DeviceControl(PortWriteOnly<u8>)` - The device control register.
/// * `DeviceAddress(PortReadOnly<u8>)` - The device address register.
#[derive(Debug, Clone)]
enum Register {
    Data(Port<u16>),
    Error(PortReadOnly<u8>),
    Features(PortWriteOnly<u8>),
    SectorCount(Port<u8>),
    Lba0(Port<u8>),
    Lba1(Port<u8>),
    Lba2(Port<u8>),
    Drive(Port<u8>),
    Status(PortReadOnly<u8>),
    Command(PortWriteOnly<u8>),

    AlternateStatus(PortReadOnly<u8>),
    DeviceControl(PortWriteOnly<u8>),
    DeviceAddress(PortReadOnly<u8>),
}

impl Register {
    /// Reads from the register.
    ///
    /// # Returns
    ///
    /// * `Result<u16, Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the register is write-only.
    fn read(&mut self) -> Result<u16, Error> {
        let value = unsafe {
            match self {
                Self::Data(port) => port.read(),

                Self::Error(port)
                | Self::DeviceAddress(port)
                | Self::Status(port)
                | Self::AlternateStatus(port) => port.read().into(),

                Self::SectorCount(port)
                | Self::Lba0(port)
                | Self::Lba1(port)
                | Self::Lba2(port)
                | Self::Drive(port) => port.read().into(),

                Self::Features(_) | Self::Command(_) | Self::DeviceControl(_) => {
                    return Err(Error::InvalidRegister(
                        "Cannot read from write-only port!".into(),
                    ))
                }
            }
        };

        Ok(value)
    }

    /// Writes to the register.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to write.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the register is read-only.
    fn write(&mut self, value: u16) -> Result<(), Error> {
        unsafe {
            match self {
                Self::Data(port) => port.write(value),

                Self::Features(port) | Self::Command(port) | Self::DeviceControl(port) => {
                    port.write(u8::try_from(value)?)
                }

                Self::SectorCount(port)
                | Self::Lba0(port)
                | Self::Lba1(port)
                | Self::Lba2(port)
                | Self::Drive(port) => port.write(u8::try_from(value)?),

                Self::Error(_)
                | Self::Status(_)
                | Self::AlternateStatus(_)
                | Self::DeviceAddress(_) => {
                    return Err(Error::InvalidRegister(
                        "Cannot write to read-only port!".into(),
                    ))
                }
            }
        };

        Ok(())
    }
}

/// The ATA bus.
///
/// # Fields
///
/// * `id` - The ID of the bus.
/// * `irq` - The IRQ of the bus.
///
/// * `data` - The data register.
/// * `error` - The error register.
/// * `features` - The features register.
/// * `sector_count` - The sector count register.
/// * `lba0` - The LBA0 register.
/// * `lba1` - The LBA1 register.
/// * `lba2` - The LBA2 register.
/// * `drive` - The drive register.
/// * `status` - The status register.
/// * `command` - The command register.
///
/// * `alternate_status` - The alternate status register.
/// * `device_address` - The device address register.
/// * `device_control` - The device control register.
#[derive(Debug, Clone)]
pub struct Bus {
    id: u8,
    irq: u8,

    data: Register,
    error: Register,
    features: Register,
    sector_count: Register,
    lba0: Register,
    lba1: Register,
    lba2: Register,
    drive: Register,
    status: Register,
    command: Register,

    alternate_status: Register,
    device_address: Register,
    device_control: Register,
}

impl Bus {
    /// Creates a new bus.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the bus.
    /// * `irq` - The IRQ of the bus.
    /// * `io_base` - The I/O base of the bus.
    /// * `ctrl_base` - The control base of the bus.
    ///
    /// # Returns
    ///
    /// * A new bus.
    #[must_use]
    pub const fn new(id: u8, irq: u8, io_base: u16, ctrl_base: u16) -> Self {
        Self {
            id,
            irq,

            data: Register::Data(Port::new(io_base)),
            error: Register::Error(PortReadOnly::new(io_base + 1)),
            features: Register::Features(PortWriteOnly::new(io_base + 1)),
            sector_count: Register::SectorCount(Port::new(io_base + 2)),
            lba0: Register::Lba0(Port::new(io_base + 3)),
            lba1: Register::Lba1(Port::new(io_base + 4)),
            lba2: Register::Lba2(Port::new(io_base + 5)),
            drive: Register::Drive(Port::new(io_base + 6)),
            status: Register::Status(PortReadOnly::new(io_base + 7)),
            command: Register::Command(PortWriteOnly::new(io_base + 7)),

            alternate_status: Register::AlternateStatus(PortReadOnly::new(ctrl_base)),
            device_address: Register::DeviceAddress(PortReadOnly::new(ctrl_base)),
            device_control: Register::DeviceControl(PortWriteOnly::new(ctrl_base)),
        }
    }

    /// Checks if the bus is floating.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the status register is invalid.
    fn floating_bus(&mut self) -> Result<bool, Error> {
        let status = self.status.read()?;

        Ok(status == 0xFF || status == 0x7F)
    }

    /// Clears the interrupt.
    ///
    /// # Returns
    ///
    /// * `Result<u8, Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the status register is invalid.
    fn clear_interrupt(&mut self) -> Result<u16, Error> {
        self.status.read()
    }

    /// Selects a drive.
    ///
    /// # Arguments
    ///
    /// * `drive` - The drive to select.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the ATA times out.
    /// * If the ATA drive does not exist.
    /// * If the `drive` register is read-only.
    fn select_drive(&mut self, drive: u8) -> Result<(), Error> {
        self.poll(Status::Busy, false)?;
        self.poll(Status::DataRequest, false)?;

        self.drive.write(u16::from(0xA0 | drive << 4))?;

        // Wait for 400 nanoseconds.
        wait(400);

        self.poll(Status::Busy, false)?;
        self.poll(Status::DataRequest, false)?;

        Ok(())
    }

    /// Checks if the bus has an error.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the status register is write-only.
    fn error(&mut self) -> Result<bool, Error> {
        Ok(self.status.read()?.get_bit(Status::Error as usize))
    }

    /// Gets the ID of the bus.
    ///
    /// # Arguments
    ///
    /// * `drive` - The drive to get the ID of.
    ///
    /// # Returns
    ///
    /// * `Result<IDResponse, Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the ATA drive does not exist.
    /// * If the ATA times out.
    /// * If the ATA drive is not a valid drive.
    fn identify_drive(&mut self, drive: u8) -> Result<DeviceType, Error> {
        if self.floating_bus()? {
            return Ok(DeviceType::None);
        }

        // Select the drive.
        self.select_drive(drive)?;
        // Clear the registers.
        self.write_cmd_params(drive, 0)?;

        // Read the status register.
        let status = self.status.read()?;
        // If the drive does not exist.
        if status == 0 {
            return Ok(DeviceType::None);
        }

        // Poll the status register until busy clears.
        self.poll(Status::Busy, false)?;

        // Determine if the drive type.
        let device_type = match (self.lba1.read()?, self.lba2.read()?) {
            (0x00, 0x00) => DeviceType::Ata({
                let mut buffer = Box::new([0; 256]);
                for chunk in buffer.iter_mut() {
                    *chunk = self.data.read()?;
                }

                buffer
            }),
            (0x14, 0xEB) => DeviceType::Atapi,
            (0x3C, 0xC3) => DeviceType::Sata,
            (_, _) => return Err(Error::Internal("Unknown ATA drive!".into())),
        };

        Ok(device_type)
    }

    /// Polls the status register.
    ///
    /// # Arguments
    ///
    /// * `bit` - The bit to poll.
    /// * `value` - The value to poll for.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the ATA times out.
    /// * If the status register is write-only.
    fn poll(&mut self, bit: Status, value: bool) -> Result<(), Error> {
        let start = uptime();

        while self.status.read()?.get_bit(bit as usize) != value {
            if uptime() - start > 1.0 {
                return Err(Error::Internal("ATA timeout.".into()));
            }

            spin_loop();
        }

        Ok(())
    }

    /// Reads from the bus.
    ///
    /// # Arguments
    ///
    /// * `drive` - The drive to read from.
    /// * `blk` - The block to read from.
    /// * `buffer` - The buffer to read into.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If PIO fails to setup for the given drive and block.
    /// * If the ATA read fails.
    fn read(&mut self, drive: u8, block: u32, buffer: &mut [u8]) -> Result<(), Error> {
        self.setup_pio(drive, block)?;
        self.write_cmd(Command::Read)?;

        for chunk in buffer.chunks_mut(2) {
            let data = self.data.read()?.to_le_bytes();

            chunk.clone_from_slice(&data);
        }

        if self.error()? {
            return Err(Error::Internal("ATA read error.".into()));
        }

        Ok(())
    }

    /// Resets the bus.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the device control register is write-only.
    fn reset(&mut self) -> Result<(), Error> {
        self.device_control.write(4)?; // set SRST.
        wait(5); // Wait for 5 nanoseconds.

        self.device_control.write(0)?; // Clear control register.
        wait(2_000); // Wait for 2 microseconds.

        Ok(())
    }

    /// Sets up PIO.
    ///
    /// # Arguments
    ///
    /// * `drive` - The drive to setup.
    /// * `block` - The block to setup.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the drive does not exist.
    /// * If the ATA times out.
    fn setup_pio(&mut self, drive: u8, block: u32) -> Result<(), Error> {
        self.select_drive(drive)?;
        self.write_cmd_params(drive, block)?;

        Ok(())
    }

    /// Writes to the bus.
    ///
    /// # Arguments
    ///
    /// * `drive` - The drive to write to.
    /// * `block` - The block to write to.
    /// * `buffer` - The buffer to write from.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If PIO fails to setup for the given drive and block.
    /// * If the ATA write fails.
    /// * If the ATA returns an error.
    /// * If the chunk is not a valid u16.
    fn write(&mut self, drive: u8, block: u32, buffer: &[u8]) -> Result<(), Error> {
        self.setup_pio(drive, block)?;
        self.write_cmd(Command::Write)?;

        for chunk in buffer.chunks(2) {
            let data = u16::from_le_bytes(chunk.try_into()?);

            self.data.write(data)?;
        }

        if self.error()? {
            return Err(Error::Internal("ATA write error!".into()));
        }

        Ok(())
    }

    /// Writes a command to the bus.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to write.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the drive does not exist.
    /// * If the ATA times out.
    fn write_cmd(&mut self, cmd: Command) -> Result<(), Error> {
        self.command.write(cmd as u16)?;

        // Wait for 400 nanoseconds.
        wait(400);

        // Ignore first read (false positive).
        self.status.read()?;
        self.clear_interrupt()?;

        // If drive does not exist.
        if self.status.read()? == 0 {
            return Err(Error::Internal("ATA drive does not exist!".into()));
        }

        self.poll(Status::Busy, false)?;
        self.poll(Status::DataRequest, true)?;

        Ok(())
    }

    /// Writes command parameters.
    ///
    /// # Arguments
    ///
    /// * `drive` - The drive to write to.
    /// * `block` - The block to write to.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the sector count register is read-only.
    fn write_cmd_params(&mut self, drive: u8, block: u32) -> Result<(), Error> {
        let lba = true;
        let mut bytes = block.to_le_bytes();

        bytes[3].set_bit(4, drive > 0);
        bytes[3].set_bit(5, true);
        bytes[3].set_bit(6, lba);
        bytes[3].set_bit(7, true);

        self.sector_count.write(1)?;
        self.lba0.write(u16::from(bytes[0]))?;
        self.lba1.write(u16::from(bytes[1]))?;
        self.lba2.write(u16::from(bytes[2]))?;
        self.drive.write(u16::from(bytes[3]))?;

        Ok(())
    }
}

/// Initializes the ATA driver.
pub fn init() {
    {
        let mut buses = BUSES.lock();

        buses.push(Bus::new(0, 14, 0x1F0, 0x3F6));
        buses.push(Bus::new(1, 15, 0x170, 0x376));
    }

    for drive in list_drives() {
        println!(
            "[INFO]: => ATA (Bus: {bus}, Disk: {disk})",
            bus = drive.bus,
            disk = drive.disk
        );
    }
}

/// Represents an ATA drive.
///
/// # Fields
///
/// * `bus` - The bus of the drive.
/// * `disk` - The disk of the drive.
///
/// * `block` - The block count of the drive.
/// * `model` - The model of the drive.
/// * `serial` - The serial number of the drive.
#[derive(Debug, Clone)]
pub struct Drive {
    pub bus: u8,
    pub disk: u8,

    block: u32,
    model: String,
    serial: String,
}

impl Drive {
    /// Gets the block count of the drive.
    ///
    /// # Returns
    ///
    /// * `u32` - The block count of the drive.
    #[must_use]
    pub const fn block_count(&self) -> u32 {
        self.block
    }

    /// Gets the block size.
    ///
    /// # Returns
    ///
    /// * `Result<u32, Error>` - The block size.
    ///
    /// # Errors
    ///
    /// * If the block size is not a valid u32.
    pub fn block_size(&self) -> Result<u32, Error> {
        Ok(u32::try_from(BLOCK_SIZE)?)
    }

    /// Opens a drive.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus of the drive.
    /// * `disk` - The disk of the drive.
    ///
    /// # Returns
    ///
    /// * `Option<Self>` - The drive, if it exists.
    pub fn open(bus: u8, disk: u8) -> Option<Self> {
        let mut buses = BUSES.lock();

        // Identify the drive.
        let Ok(DeviceType::Ata(result)) = buses[bus as usize].identify_drive(disk) else {
            return None;
        };

        let buffer = result.map(u16::to_le_bytes).concat();
        let block = u32::from_be_bytes(buffer[120..124].try_into().ok()?).rotate_left(16);
        let model = String::from_utf8_lossy(&buffer[54..94]).trim().into();
        let serial = String::from_utf8_lossy(&buffer[20..40]).trim().into();

        Some(Self {
            bus,
            disk,
            block,
            model,
            serial,
        })
    }

    /// Gets the formatted size of the drive.
    ///
    /// # Returns
    ///
    /// * `Result<(usize, String), Error>` - The formatted size of the drive.
    ///
    /// # Errors
    ///
    /// * If the block size is not a valid u32.
    fn formatted_size(&self) -> Result<(usize, String), Error> {
        let count = self.block_count() as usize;
        let size = self.block_size()? as usize;

        let bytes = size * count;

        Ok(if bytes >> 20 < 1_000 {
            (bytes >> 20, String::from("MB"))
        } else {
            (bytes >> 30, String::from("GB"))
        })
    }
}

/// Lists the drives.
///
/// # Returns
///
/// * `Vec<Drive>` - The drives.
#[must_use]
pub fn list_drives() -> Vec<Drive> {
    let mut drives = Vec::new();
    for bus in 0..2 {
        for disk in 0..2 {
            if let Some(drive) = Drive::open(bus, disk) {
                drives.push(drive);
            }
        }
    }

    drives
}

/// Reads from a drive.
///
/// # Arguments
///
/// * `bus` - The bus of the drive.
/// * `drive` - The drive to read from.
/// * `blk` - The block to read from.
/// * `buffer` - The buffer to read into.
///
/// # Returns
///
/// * `Result<(), Error>` - The result of the operation.
///
/// # Errors
///
/// * If the drive does not exist.
/// * If the ATA times out.
/// * If the ATA read fails.
/// * If the ATA returns an error.
pub fn read(bus: u8, drive: u8, block: u32, buffer: &mut [u8]) -> Result<(), Error> {
    let mut buses = BUSES.lock();

    buses[bus as usize].read(drive, block, buffer)
}

/// Writes to a drive.
///
/// # Arguments
///
/// * `bus` - The bus of the drive.
/// * `drive` - The drive to write to.
/// * `block` - The block to write to.
/// * `buffer` - The buffer to write from.
///
/// # Returns
///
/// * `Result<(), Error>` - The result of the operation.
///
/// # Errors
///
/// * If the drive does not exist.
/// * If the ATA times out.
/// * If the ATA write fails.
/// * If the ATA returns an error.
pub fn write(bus: u8, drive: u8, block: u32, buffer: &[u8]) -> Result<(), Error> {
    let mut buses = BUSES.lock();

    buses[bus as usize].write(drive, block, buffer)
}
