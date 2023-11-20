use crate::println;

pub mod ata;

/// Initializes the device drivers.
pub fn init() {
    println!("[INFO]: Initializing the ATA driver...");
    ata::init();
}
