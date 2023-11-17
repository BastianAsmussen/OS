use crate::dev::ata::bus::Bus;
use crate::dev::ata::register::RegisterHandler;

/// The ATA drive.
///
/// # Fields
///
/// * `bus` - The bus the drive is on.
/// * `drive` - The drive number.
/// * `register_handler` - The register handler.
///
/// * `io_base` - The I/O base port.
/// * `control_base` - The control base port.
#[derive(Debug)]
pub struct Drive {
    pub bus: Bus,
    pub drive: u8,
    pub register_handler: RegisterHandler,

    io_base: u16,
    control_base: u16,
}

impl Drive {
    /// Creates a new drive.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus the drive is on.
    /// * `drive` - The drive number.
    /// * `io_base` - The I/O base port.
    /// * `control_base` - The control base port.
    ///
    /// # Returns
    ///
    /// * `Self` - The drive.
    pub fn new(bus: Bus, drive: u8, io_base: u16, control_base: u16) -> Self {
        Self {
            bus,
            drive,
            register_handler: RegisterHandler::new(io_base, control_base),
            io_base,
            control_base,
        }
    }
}
