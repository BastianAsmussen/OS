use crate::dev::ata::bus::Bus;
use crate::errors::Error;

pub mod bus;
pub mod registers;

pub fn init() -> Result<(), Error> {
    let mut bus = Bus::new(0, 14, 0x1F0, 0x3F6);

    Ok(())
}
