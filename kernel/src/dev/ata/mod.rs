use crate::errors::Error;

pub mod bus;
pub mod registers;

/// Initializes the ATA driver.
pub fn init() -> Result<(), Error> {
    todo!("ATA initialization isn't yet implemented!")
}
