use crate::sys::time;

/// Gets the uptime of the sys.
///
/// # Returns
///
/// * `f64` - The uptime of the system in seconds.
#[must_use]
pub fn uptime() -> f64 {
    time::pit_interval() * time::tick() as f64
}
