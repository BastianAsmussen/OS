use pic8259::ChainedPics;
use spin::Mutex;

/// The first PIC offset, used for remapping.
pub const PIC_1_OFFSET: u8 = 32;

/// The second PIC offset, used for remapping.
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// The Programmable Interrupt Controller.
///
/// # Notes
///
/// * This is a spinlock because it is shared between multiple CPUs.
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
