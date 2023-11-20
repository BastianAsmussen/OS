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

pub const BLKSIZE: usize = 512;

lazy_static! {
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

/// Represents an ID response.
///
/// # Variants
///
/// * `Ata(Box<[u16; 256]>)` - The ATA response.
/// * `Atapi` - The ATAPI response.
/// * `Sata` - The SATA response.
/// * `None` - No response.
#[derive(Debug)]
enum IDResponse {
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

/// The ATA bus.
///
/// # Fields
///
/// * `id` - The ID of the bus.
/// * `irq` - The IRQ of the bus.
///
/// * `data_reg` - The data register.
/// * `err_reg` - The error register.
/// * `features_reg` - The features register.
/// * `sector_count_reg` - The sector count register.
/// * `lba0_reg` - The LBA0 register.
/// * `lba1_reg` - The LBA1 register.
/// * `lba2_reg` - The LBA2 register.
/// * `drive_reg` - The drive register.
/// * `status_reg` - The status register.
/// * `cmd_reg` - The command register.
///
/// * `alt_status_reg` - The alternate status register.
/// * `device_ctrl_reg` - The device control register.
/// * `device_addr_reg` - The device address register.
#[derive(Debug, Clone)]
pub struct Bus {
    id: u8,
    irq: u8,

    data_reg: Port<u16>,
    err_reg: PortReadOnly<u8>,
    features_reg: PortWriteOnly<u8>,
    sector_count_reg: Port<u8>,
    lba0_reg: Port<u8>,
    lba1_reg: Port<u8>,
    lba2_reg: Port<u8>,
    drive_reg: Port<u8>,
    status_reg: PortReadOnly<u8>,
    cmd_reg: PortWriteOnly<u8>,

    alt_status_reg: PortReadOnly<u8>,
    device_ctrl_reg: PortWriteOnly<u8>,
    device_addr_reg: PortReadOnly<u8>,
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

            data_reg: Port::new(io_base),
            err_reg: PortReadOnly::new(io_base + 1),
            features_reg: PortWriteOnly::new(io_base + 1),
            sector_count_reg: Port::new(io_base + 2),
            lba0_reg: Port::new(io_base + 3),
            lba1_reg: Port::new(io_base + 4),
            lba2_reg: Port::new(io_base + 5),
            drive_reg: Port::new(io_base + 6),
            status_reg: PortReadOnly::new(io_base + 7),
            cmd_reg: PortWriteOnly::new(io_base + 7),

            alt_status_reg: PortReadOnly::new(ctrl_base),
            device_addr_reg: PortReadOnly::new(ctrl_base),
            device_ctrl_reg: PortWriteOnly::new(ctrl_base + 1),
        }
    }

    /// Checks if the bus is floating.
    ///
    /// # Returns
    ///
    /// * `bool` - true if the bus is floating, false otherwise.
    fn floating_bus(&mut self) -> bool {
        let status = self.status();

        status == 0xFF || status == 0x7F
    }

    /// Clears the interrupt.
    ///
    /// # Returns
    ///
    /// * `u8` - The status register.
    fn clear_interrupt(&mut self) -> u8 {
        unsafe { self.status_reg.read() }
    }

    /// Reads the data register.
    ///
    /// # Returns
    ///
    /// * `u16` - The PIO data bytes.
    fn read_data(&mut self) -> u16 {
        unsafe { self.data_reg.read() }
    }

    /// Writes to the data register.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to write.
    fn write_data(&mut self, data: u16) {
        unsafe { self.data_reg.write(data) }
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
    fn select_drive(&mut self, drive: u8) -> Result<(), Error> {
        self.poll(Status::Busy, false)?;
        self.poll(Status::DataRequest, false)?;

        unsafe {
            self.drive_reg.write(0xA0 | drive << 4);
        }

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
    /// * `bool` - true if the bus has an error, false otherwise.
    fn error(&mut self) -> bool {
        self.status().get_bit(Status::Error as usize)
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
    fn identify_drive(&mut self, drive: u8) -> Result<IDResponse, Error> {
        if self.floating_bus() {
            return Ok(IDResponse::None);
        }

        // Select the drive.
        self.select_drive(drive)?;
        // Clear the registers.
        self.write_cmd_params(drive, 0);

        // Read the status register.
        let status = self.status();
        // If the drive does not exist.
        if status == 0 {
            return Ok(IDResponse::None);
        }

        // Poll the status register until busy clears.
        self.poll(Status::Busy, false)?;

        // Determine if the drive type.
        match (self.lba1(), self.lba2()) {
            (0x00, 0x00) => Ok(IDResponse::Ata(Box::from(
                [(); 256].map(|()| self.read_data()),
            ))),
            (0x14, 0xEB) => Ok(IDResponse::Atapi),
            (0x3C, 0xC3) => Ok(IDResponse::Sata),
            (_, _) => Err(Error::Internal("Unknown ATA drive!".into())),
        }
    }

    /// Reads the LBA1 register.
    ///
    /// # Returns
    ///
    /// * `u8` - The LBA1 register.
    fn lba1(&mut self) -> u8 {
        unsafe { self.lba1_reg.read() }
    }

    /// Reads the LBA2 register.
    ///
    /// # Returns
    ///
    /// * `u8` - The LBA2 register.
    fn lba2(&mut self) -> u8 {
        unsafe { self.lba2_reg.read() }
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
    fn poll(&mut self, bit: Status, value: bool) -> Result<(), Error> {
        let start = uptime();

        while self.status().get_bit(bit as usize) != value {
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
            let data = self.read_data().to_le_bytes();

            chunk.clone_from_slice(&data);
        }

        if self.error() {
            return Err(Error::Internal("ATA read error.".into()));
        }

        Ok(())
    }

    /// Resets the bus.
    fn reset(&mut self) {
        unsafe {
            self.device_ctrl_reg.write(4); // set SRST.
            wait(5); // Wait for 5 nanoseconds.

            self.device_ctrl_reg.write(0); // Clear control register.
            wait(2_000); // Wait for 2 microseconds.
        }
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
        self.write_cmd_params(drive, block);

        Ok(())
    }

    /// Reads the status register.
    ///
    /// # Returns
    ///
    /// * `u8` - The status register.
    fn status(&mut self) -> u8 {
        unsafe { self.alt_status_reg.read() }
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

            self.write_data(data);
        }

        if self.error() {
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
        unsafe {
            self.cmd_reg.write(cmd as u8);
        }

        // Wait for 400 nanoseconds.
        wait(400);

        // Ignore first read (false positive).
        self.status();
        self.clear_interrupt();

        // If drive does not exist.
        if self.status() == 0 {
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
    fn write_cmd_params(&mut self, drive: u8, block: u32) {
        let lba = true;
        let mut bytes = block.to_le_bytes();

        bytes[3].set_bit(4, drive > 0);
        bytes[3].set_bit(5, true);
        bytes[3].set_bit(6, lba);
        bytes[3].set_bit(7, true);

        unsafe {
            self.sector_count_reg.write(1);
            self.lba0_reg.write(bytes[0]);
            self.lba1_reg.write(bytes[1]);
            self.lba2_reg.write(bytes[2]);
            self.drive_reg.write(bytes[3]);
        }
    }
}

/// Initializes the ATA driver.
pub fn init() {
    {
        let mut buses = BUSES.lock();

        buses.push(Bus::new(0, 14, 0x1F0, 0x3F6));
        buses.push(Bus::new(1, 15, 0x170, 0x376));
    }

    for drive in ls() {
        println!(
            "[INFO]: ATA {bus} {disk} {drive:#?}",
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
        Ok(u32::try_from(BLKSIZE)?)
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
        let Ok(IDResponse::Ata(result)) = buses[bus as usize].identify_drive(disk) else {
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
pub fn ls() -> Vec<Drive> {
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
