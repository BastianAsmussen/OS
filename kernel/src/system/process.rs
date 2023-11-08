use crate::allocator::HEAP_START;
use crate::errors::Error;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use lazy_static::lazy_static;
use object::{Object, ObjectSegment};
use spin::RwLock;
use x86_64::structures::idt::InterruptStackFrameValue;
use x86_64::VirtAddr;

/// The code address is the start of the code segment in memory.
///
/// # Notes
///
/// * This is set to the start of the heap, plus 4 GiB.
pub static CODE_ADDRESS: AtomicU64 = AtomicU64::new((HEAP_START as u64) + (1 << 32));

/// The page size, which is how we size the stack.
///
/// # Notes
///
/// * This is 4 KiB.
pub const PAGE_SIZE: usize = 4 * 1024;

/// The first 4 bytes of an ELF file are the magic number.
///
/// # Notes
///
/// * This is the magic number for 64-bit ELF files.
const ELF_MAGIC_NUMBER: u32 = 0x7F_45_4C_46;

/// The maximum number of file handles.
const MAX_FILE_HANDLES: usize = 256;

/// The maximum number of processes.
const MAX_PROCESSES: usize = 16;

lazy_static! {
    /// The current process ID.
    pub static ref PROCESS_ID: AtomicUsize = AtomicUsize::new(0);
    /// The maximum process ID.
    pub static ref MAX_PROCESS_ID: AtomicUsize = AtomicUsize::new(1);
    /// The process table.
    pub static ref PROCESS_TABLE: RwLock<[Process; MAX_PROCESSES]> = RwLock::new([(); MAX_PROCESSES].map(|_| Process::new(0)));
}

/// A process.
///
/// # Fields
///
/// * `id`: The process ID.
/// * `code_addr`: The code address, i.e. the start of the code segment in memory.
/// * `code_size`: The size of the code segment in bytes.
/// * `entry_point`: The entry point of the process.
/// * `data`: The process data.
/// * `registers`: The state of the process registers.
/// * `stack_frame`: The process stack frame.
#[derive(Debug)]
pub struct Process {
    id: usize,
    code_addr: usize,
    code_size: usize,
    entry_point: usize,
    data: ProcessData,
    registers: Registers,
    stack_frame: InterruptStackFrameValue,
}

impl Process {
    /// Creates a new `Process`.
    ///
    /// # Arguments
    ///
    /// * `id` - The process ID.
    pub fn new(id: usize) -> Self {
        let stack_frame = InterruptStackFrameValue {
            code_segment: 0,
            cpu_flags: 0,
            instruction_pointer: VirtAddr::new(0),
            stack_pointer: VirtAddr::new(0),
            stack_segment: 0,
        };

        Self {
            id,
            code_addr: 0,
            code_size: 0,
            entry_point: 0,
            data: ProcessData::default(),
            registers: Registers::default(),
            stack_frame,
        }
    }

    /// Spawns a new process.
    ///
    /// # Arguments
    ///
    /// * `binary` - The bytes of the binary to spawn.
    ///
    /// # Returns
    ///
    /// * `Result<usize, Error>` - The process ID, or an error.
    pub fn spawn(binary: &[u8]) -> Result<usize, Error> {
        let code_size = 1024 * PAGE_SIZE as u64;
        let code_addr = CODE_ADDRESS.fetch_add(code_size, Ordering::SeqCst);

        // Allocate the code segment on a new page.
        crate::memory::alloc_page(code_addr, code_size)?;

        let code_ptr = code_addr as *mut u8;

        // Extract the data from the binary.
        let binary = Binary::new(binary, code_ptr)?;

        let mut table = PROCESS_TABLE.write();
        let Some(parent) = table.get_mut(PROCESS_ID.load(Ordering::SeqCst)) else {
            return Err(Error::Internal("Failed to get parent process ID!".into()));
        };
        let data = *parent.data;
        let registers = *parent.registers;
        let stack_frame = *parent.stack_frame;

        let id = MAX_PROCESS_ID.fetch_add(1, Ordering::SeqCst);
        let process = Process {
            id,
            code_addr: code_ptr as usize,
            code_size: binary.code_ptr as usize,
            entry_point: binary.entry_point as usize,
            data,
            registers,
            stack_frame,
        };

        Ok(id)
    }
}

/// The data of a process.
#[derive(Debug, Default)]
struct ProcessData;

/// The state of the process registers.
///
/// # Fields
///
/// * `rax`: The `rax` register.
/// * `rcx`: The `rcx` register.
/// * `rdx`: The `rdx` register.
/// * `rdi`: The `rdi` register.
/// * `rsi`: The `rsi` register.
/// * `r11`: The `r11` register.
/// * `r10`: The `r10` register.
/// * `r9`: The `r9` register.
/// * `r8`: The `r8` register.
#[derive(Debug, Default)]
pub struct Registers {
    pub rax: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
}

/// A binary.
///
/// # Fields
///
/// * `entry_point`: The entry point of the binary.
/// * `code_ptr`: The pointer to the code of the binary.
struct Binary {
    entry_point: u64,
    code_ptr: *mut u8,
}

impl Binary {
    /// Creates a new `Binary`.
    ///
    /// # Arguments
    ///
    /// * `binary` - The bytes of the binary.
    /// * `code_address` - The address of the code segment in memory.
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - The `Binary`, or an error.
    pub fn new(binary: &[u8], code_address: *mut u8) -> Result<Self, Error> {
        let mut entry_point = 0;
        let code_ptr = code_address;

        Self::extract_data(binary, code_ptr, &mut entry_point)?;

        Ok(Self {
            entry_point,
            code_ptr,
        })
    }

    /// Whether the given binary is an ELF binary or not.
    ///
    /// # Arguments
    ///
    /// * `binary` - The bytes of the binary.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether the given binary is an ELF binary or not.
    fn is_elf_binary(binary: &[u8]) -> bool {
        binary.len() > 4 && binary[0..4] == ELF_MAGIC_NUMBER.to_le_bytes()
    }

    /// Extracts the data from the binary.
    ///
    /// # Arguments
    ///
    /// * `binary` - The bytes of the binary.
    /// * `code_ptr` - The pointer to the code of the binary.
    /// * `entry_point` - The entry point of the binary.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - A result indicating whether the data was extracted successfully or not.
    ///
    /// # Notes
    ///
    /// * If the binary is an ELF binary, the code will be extracted from the ELF segments.
    fn extract_data(binary: &[u8], code_ptr: *mut u8, entry_point: &mut u64) -> Result<(), Error> {
        // Check if the binary is an ELF binary, otherwise assume it's a raw binary.
        if Self::is_elf_binary(&binary) {
            let file = object::File::parse(binary)?;

            entry_point = file.entry();

            for segment in file.segments() {
                let addr = segment.address() as usize;
                let size = segment.size() as usize;
                // If the segment is not a loadable segment, skip it.
                let Ok(data) = segment.data() else {
                    continue;
                };

                data.iter()
                    .enumerate()
                    .for_each(|(i, op)| unsafe { core::ptr::write(code_ptr.add(addr + i), *op) });
            }
        } else {
            binary
                .iter()
                .enumerate()
                .for_each(|(i, op)| unsafe { core::ptr::write(code_ptr.add(i), *op) });
        }

        Ok(())
    }
}
