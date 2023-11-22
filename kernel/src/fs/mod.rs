use crate::fs::fat::Fat;
use crate::println;

pub mod fat;

/// Initializes the file system.
/// 
/// # Returns
/// 
/// * The FAT file system.
#[must_use]
pub fn init() -> Fat {
    println!("[INFO]: Initializing the FAT file system...");
    fat::init()
}
