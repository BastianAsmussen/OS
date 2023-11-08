use alloc::format;
use alloc::string::String;
use core::alloc::LayoutError;
use thiserror_no_std::Error;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::Size4KiB;

/// An error representation.
///
/// # Variants
///
/// * `Internal` - An internal error.
/// * `Mapping` - A mapping error.
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
