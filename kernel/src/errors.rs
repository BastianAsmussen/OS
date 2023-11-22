use crate::sys::task::Identifier;
use alloc::format;
use alloc::string::String;
use core::alloc::LayoutError;
use core::array::TryFromSliceError;
use core::num::TryFromIntError;
use thiserror_no_std::Error;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::Size4KiB;

/// An error representation.
///
/// # Variants
///
/// * `Internal` - An internal error.
/// * `Mapping` - A mapping error.
/// * `OutOfMemory` - An out of memory error.
/// * `MemoryLayout` - A memory layout error.
/// * `InvalidRegister` - An invalid register error.
/// * `InvalidAddress` - An invalid address error.
/// * `Conversion` - A conversion error.
/// * `Task` - A task error.
/// * `FileSystem` - A file system error.
#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("Internal Error: {0}")]
    Internal(String),
    #[error("Mapping Error: {0}")]
    Mapping(String),
    #[error("Out of Memory Error: {0}")]
    OutOfMemory(String),
    #[error("Memory Layout Error: {0}")]
    MemoryLayout(String),
    #[error("Invalid Register Error: {0}")]
    InvalidRegister(String),
    #[error("ATA Error: {0}")]
    ATA(String),
    #[error("Conversion Error: {0}")]
    Conversion(String),
    #[error("Task Error: {0}")]
    Task(String),
    #[error("File System Error: {0}")]
    FileSystem(String),
}

impl From<MapToError<Size4KiB>> for Error {
    fn from(error: MapToError<Size4KiB>) -> Self {
        Self::Mapping(format!("{error:#?}"))
    }
}

impl From<LayoutError> for Error {
    fn from(error: LayoutError) -> Self {
        Self::MemoryLayout(format!("{error:#?}"))
    }
}

impl From<TryFromIntError> for Error {
    fn from(error: TryFromIntError) -> Self {
        Self::Conversion(format!("{error:#?}"))
    }
}

impl From<TryFromSliceError> for Error {
    fn from(error: TryFromSliceError) -> Self {
        Self::Conversion(format!("{error:#?}"))
    }
}

impl From<Identifier> for Error {
    fn from(error: Identifier) -> Self {
        Self::Task(format!("{error:#?}"))
    }
}
