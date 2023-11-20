use crate::errors::Error;
use crate::println;

pub mod ata;
pub mod net;

/// Initializes the device drivers.
///
/// # Returns
///
/// * `Result<(), Error>` - The result of the initialization.
///
/// # Errors
///
/// * If the ATA driver fails to initialize.
/// * If the network driver fails to initialize.
pub fn init() -> Result<(), Error> {
    println!("[INFO]: Initializing the ATA driver...");
    ata::init();

    // println!("[INFO]: Initializing the network driver...");
    // net::init()?;

    Ok(())
}
