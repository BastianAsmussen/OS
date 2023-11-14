use crate::println;
use crate::sys::time::rtc::RTC;
use alloc::format;

/// Print the clock on RTC update.
///
/// # Arguments
///
/// * `rtc` - The RTC struct.
pub fn print(rtc: &RTC) {
    let date = format!(
        "{day:02}/{month:02}/{century:02}{year:02}",
        day = rtc.day,
        month = rtc.month,
        century = rtc.century,
        year = rtc.year
    );
    let time = format!(
        "{hour:02}:{minute:02}:{second:02}",
        hour = rtc.hours,
        minute = rtc.minutes,
        second = rtc.seconds
    );

    println!("[INFO]: {date} @ {time}", date = date, time = time);
}
