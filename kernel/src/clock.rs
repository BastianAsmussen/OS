use crate::time;

/// Gets the uptime of the system.
///
/// # Returns
///
/// * `f64` - The uptime of the system in seconds.
#[must_use]
pub fn uptime() -> f64 {
    time::interval() * time::tick() as f64
}
