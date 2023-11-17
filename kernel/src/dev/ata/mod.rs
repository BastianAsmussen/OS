pub mod bus;
pub mod drive;
pub mod register;

use crate::errors::Error;

/// Initializes the ATA driver.
pub fn init() -> Result<(), Error> {
    Ok(())
}
