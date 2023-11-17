use crate::sys::time::rtc::RTC;

/// System calls are used to interact with the kernel.
///
/// # Variants
///
/// * `Sleep` - Sleep for a specified amount of time.
/// * `Uptime` - Get the uptime of the system.
/// * `RTC` - Get the current time from the RTC.
/// * `Unknown` - An unknown system call.
#[derive(Debug)]
pub enum Call {
    Sleep = 0x1,
    Uptime = 0x2,
    RTC = 0x3,
    Unknown = 0x4,
}

/// Dispatches a system call.
///
/// # Arguments
///
/// * `call` - The system call.
/// * `args` - The arguments for the system call.
///
/// # Returns
///
/// * `Option<usize>` - The return value of the system call.
#[must_use]
pub fn dispatch(call: &Call, args: &[usize]) -> Option<usize> {
    match call {
        Call::Sleep => {
            let duration = args[0];

            crate::sys::time::sleep(duration as f64);

            Some(0)
        }
        Call::Uptime => {
            let uptime = crate::sys::time::clock::uptime();

            Some(uptime as usize)
        }
        Call::RTC => {
            let rtc = RTC::new();
            let millis = rtc.as_millis();

            usize::try_from(millis).ok()
        }
        Call::Unknown => None,
    }
}
