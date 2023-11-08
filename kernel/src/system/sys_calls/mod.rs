use core::arch::asm;

pub mod services;

/// The system call numbers.
///
/// # Variants
///
/// * `Exit` - The exit system call, which exits the current process.
/// * `Spawn` - The spawn system call, which spawns a new process.
/// * `Read` - The read system call, which reads from a file.
/// * `Write` - The write system call, which writes to a file.
/// * `Open` - The open system call, which opens a file.
/// * `Close` - The close system call, which closes a file.
/// * `Information` - The information system call, which gets information about a file.
/// * `Duplicate` - The duplicate system call, which duplicates a file descriptor.
/// * `Sleep` - The sleep system call, which sleeps for a certain amount of time.
/// * `Uptime` - The uptime system call, which gets the uptime.
/// * `RealTime` - The real time system call, which gets the real time.
/// * `Invalid` - The unimplemented system call, which is invalid.
pub enum SystemCall {
    Exit = 0x1,
    Spawn = 0x2,
    Read = 0x3,
    Write = 0x4,
    Open = 0x5,
    Close = 0x6,
    Information = 0x7,
    Duplicate = 0x8,
    Sleep = 0x9,
    Uptime = 0xA,
    RealTime = 0xB,
    Invalid = 0x0,
}

/// Exits the current process.
///
/// # Arguments
///
/// * `code` - The exit code of the process.
///
/// # Returns
///
/// * `usize` - The result of the system call.
#[must_use]
pub fn exit(code: usize) -> usize {
    let result;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") code,
            lateout("rax") result,
        );
    }

    result
}

/// Spawns a new process.
///
/// # Arguments
///
/// * `addr` - The address of the process, in memory.
/// * `len` - The length of the process, in elements.
///
/// # Returns
///
/// * `usize` - The result of the system call.
#[must_use]
pub fn spawn(addr: usize, len: usize) -> usize {
    let ptr = todo!("Get the pointer to the process in memory.");
    let path = unsafe { core::slice::from_raw_parts(ptr, len) };

    let result;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") SystemCall::Spawn as usize,
            in("rdi") path.as_ptr(),
            in("rsi") path.len(),
            lateout("rax") result,
        );
    }

    result
}
