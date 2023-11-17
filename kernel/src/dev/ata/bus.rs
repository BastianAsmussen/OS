/// The ATA bus.
///
/// # Variants
///
/// * `Primary` - The primary bus.
/// * `Secondary` - The secondary bus.
#[derive(Debug)]
pub enum Bus {
    Primary,
    Secondary,
}
