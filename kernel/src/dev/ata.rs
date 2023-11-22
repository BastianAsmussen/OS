// ; do a singletasking PIO ATA read
// ; inputs: ebx = # of sectors to read, edi -> dest buffer, esi -> driverdata struct, ebp = 4b LBA
// ; Note: ebp is a "relative" LBA -- the offset from the beginning of the partition
// ; outputs: ebp, edi incremented past read; ebx = 0
// ; flags: zero flag set on success, carry set on failure (redundant)
// read_ata_st:
// 	push edx
// 	push ecx
// 	push eax
// 	test ebx, ebx			; # of sectors < 0 is a "reset" request from software
// 	js short .reset
// 	cmp ebx, 0x3fffff		; read will be bigger than 2GB? (error)
// 	stc
// 	jg short .r_don
// 	mov edx, [esi + dd_prtlen]	; get the total partition length (sectors)
// 	dec edx				; (to avoid handling "equality" case)
// 	cmp edx, ebp			; verify ebp is legal (within partition limit)
// 	jb short .r_don			; (carry is set automatically on an error)
// 	cmp edx, ebx			; verify ebx is legal (forget about the ebx = edx case)
// 	jb short .r_don
// 	sub edx, ebx			; verify ebp + ebx - 1 is legal
// 	inc edx
// 	cmp edx, ebp			; (the test actually checks ebp <= edx - ebx + 1)
// 	jb short .r_don
// 	mov dx, [esi + dd_dcr]		; dx = alt status/DCR
// 	in al, dx			; get the current status
// 	test al, 0x88			; check the BSY and DRQ bits -- both must be clear
// 	je short .stat_ok
// .reset:
// 	call srst_ata_st
// 	test ebx, ebx			; bypass any read on a "reset" request
// 	jns short .stat_ok
// 	xor ebx, ebx			; force zero flag on, carry clear
// 	jmp short .r_don
// .stat_ok:
// ; preferentially use the 28bit routine, because it's a little faster
// ; if ebp > 28bit or esi.stLBA > 28bit or stLBA+ebp > 28bit or stLBA+ebp+ebx > 28bit, use 48 bit
// 	cmp ebp, 0xfffffff
// 	jg short .setreg
// 	mov eax, [esi + dd_stLBA]
// 	cmp eax, 0xfffffff
// 	jg short .setreg
// 	add eax, ebp
// 	cmp eax, 0xfffffff
// 	jg short .setreg
// 	add eax, ebx
// 	cmp eax, 0xfffffff
// .setreg:
// 	mov dx, [esi + dd_tf]		; dx = IO port base ("task file")
// 	jle short .read28		; test the flags from the eax cmp's above
// .read48:
// 	test ebx, ebx		; no more sectors to read?
// 	je short .r_don
// 	call pio48_read		; read up to 256 more sectors, updating registers
// 	je short .read48	; if successful, is there more to read?
// 	jmp short .r_don
// .read28:
// 	test ebx, ebx		; no more sectors to read?
// 	je short .r_don
// 	call pio28_read		; read up to 256 more sectors, updating registers
// 	je short .read28	; if successful, is there more to read?
// .r_don:
// 	pop eax
// 	pop ecx
// 	pop edx
// 	ret
//
//
// ;ATA PI0 28bit singletasking disk read function (up to 256 sectors)
// ; inputs: ESI -> driverdata info, EDI -> destination buffer
// ; BL = sectors to read, DX = base bus I/O port (0x1F0, 0x170, ...), EBP = 28bit "relative" LBA
// ; BSY and DRQ ATA status bits must already be known to be clear on both slave and master
// ; outputs: data stored in EDI; EDI and EBP advanced, EBX decremented
// ; flags: on success Zero flag set, Carry clear
// pio28_read:
// 	add ebp, [esi + dd_stLBA]	; convert relative LBA to absolute LBA
// 	mov ecx, ebp			; save a working copy
// 	mov al, bl		; set al= sector count (0 means 256 sectors)
// 	or dl, 2		; dx = sectorcount port -- usually port 1f2
// 	out dx, al
// 	mov al, cl		; ecx currently holds LBA
// 	inc edx			; port 1f3 -- LBAlow
// 	out dx, al
// 	mov al, ch
// 	inc edx			; port 1f4 -- LBAmid
// 	out dx, al
// 	bswap ecx
// 	mov al, ch		; bits 16 to 23 of LBA
// 	inc edx			; port 1f5 -- LBAhigh
// 	out dx, al
// 	mov al, cl			; bits 24 to 28 of LBA
// 	or al, byte [esi + dd_sbits]	; master/slave flag | 0xe0
// 	inc edx				; port 1f6 -- drive select
// 	out dx, al
//
// 	inc edx			; port 1f7 -- command/status
// 	mov al, 0x20		; send "read" command to drive
// 	out dx, al
//
// ; ignore the error bit for the first 4 status reads -- ie. implement 400ns delay on ERR only
// ; wait for BSY clear and DRQ set
// 	mov ecx, 4
// .lp1:
// 	in al, dx		; grab a status byte
// 	test al, 0x80		; BSY flag set?
// 	jne short .retry
// 	test al, 8		; DRQ set?
// 	jne short .data_rdy
// .retry:
// 	dec ecx
// 	jg short .lp1
// ; need to wait some more -- loop until BSY clears or ERR sets (error exit if ERR sets)
//
// .pior_l:
// 	in al, dx		; grab a status byte
// 	test al, 0x80		; BSY flag set?
// 	jne short .pior_l	; (all other flags are meaningless if BSY is set)
// 	test al, 0x21		; ERR or DF set?
// 	jne short .fail
// .data_rdy:
// ; if BSY and ERR are clear then DRQ must be set -- go and read the data
// 	sub dl, 7		; read from data port (ie. 0x1f0)
// 	mov cx, 256
// 	rep insw		; gulp one 512b sector into edi
// 	or dl, 7		; "point" dx back at the status register
// 	in al, dx		; delay 400ns to allow drive to set new values of BSY and DRQ
// 	in al, dx
// 	in al, dx
// 	in al, dx
//
// ; After each DRQ data block it is mandatory to either:
// ; receive and ack the IRQ -- or poll the status port all over again
//
// 	inc ebp			; increment the current absolute LBA
// 	dec ebx			; decrement the "sectors to read" count
// 	test bl, bl		; check if the low byte just turned 0 (more sectors to read?)
// 	jne short .pior_l
//
// 	sub dx, 7		; "point" dx back at the base IO port, so it's unchanged
// 	sub ebp, [esi + dd_stLBA]	; convert absolute lba back to relative
// ; "test" sets the zero flag for a "success" return -- also clears the carry flag
// 	test al, 0x21		; test the last status ERR bits
// 	je short .done
// .fail:
// 	stc
// .done:
// 	ret
//
//
// ;ATA PI0 33bit singletasking disk read function (up to 64K sectors, using 48bit mode)
// ; inputs: bx = sectors to read (0 means 64K sectors), edi -> destination buffer
// ; esi -> driverdata info, dx = base bus I/O port (0x1F0, 0x170, ...), ebp = 32bit "relative" LBA
// ; BSY and DRQ ATA status bits must already be known to be clear on both slave and master
// ; outputs: data stored in edi; edi and ebp advanced, ebx decremented
// ; flags: on success Zero flag set, Carry clear
// pio48_read:
// 	xor eax, eax
// 	add ebp, [esi + dd_stLBA]	; convert relative LBA to absolute LBA
// ; special case: did the addition overflow 32 bits (carry set)?
// 	adc ah, 0			; if so, ah = LBA byte #5 = 1
// 	mov ecx, ebp			; save a working copy of 32 bit absolute LBA
//
// ; for speed purposes, never OUT to the same port twice in a row -- avoiding it is messy but best
// ;outb (0x1F2, sectorcount high)
// ;outb (0x1F3, LBA4)
// ;outb (0x1F4, LBA5)			-- value = 0 or 1 only
// ;outb (0x1F5, LBA6)			-- value = 0 always
// ;outb (0x1F2, sectorcount low)
// ;outb (0x1F3, LBA1)
// ;outb (0x1F4, LBA2)
// ;outb (0x1F5, LBA3)
// 	bswap ecx		; make LBA4 and LBA3 easy to access (cl, ch)
// 	or dl, 2		; dx = sectorcount port -- usually port 1f2
// 	mov al, bh		; sectorcount -- high byte
// 	out dx, al
// 	mov al, cl
// 	inc edx
// 	out dx, al		; LBA4 = LBAlow, high byte (1f3)
// 	inc edx
// 	mov al, ah		; LBA5 was calculated above
// 	out dx, al		; LBA5 = LBAmid, high byte (1f4)
// 	inc edx
// 	mov al, 0		; LBA6 is always 0 in 32 bit mode
// 	out dx, al		; LBA6 = LBAhigh, high byte (1f5)
//
// 	sub dl, 3
// 	mov al, bl		; sectorcount -- low byte (1f2)
// 	out dx, al
// 	mov ax, bp		; get LBA1 and LBA2 into ax
// 	inc edx
// 	out dx, al		; LBA1 = LBAlow, low byte (1f3)
// 	mov al, ah		; LBA2
// 	inc edx
// 	out dx, al		; LBA2 = LBAmid, low byte (1f4)
// 	mov al, ch		; LBA3
// 	inc edx
// 	out dx, al		; LBA3 = LBAhigh, low byte (1f5)
//
// 	mov al, byte [esi + dd_sbits]	; master/slave flag | 0xe0
// 	inc edx
// 	and al, 0x50		; get rid of extraneous LBA28 bits in drive selector
// 	out dx, al		; drive select (1f6)
//
// 	inc edx
// 	mov al, 0x24		; send "read ext" command to drive
// 	out dx, al		; command (1f7)
//
// ; ignore the error bit for the first 4 status reads -- ie. implement 400ns delay on ERR only
// ; wait for BSY clear and DRQ set
// 	mov ecx, 4
// .lp1:
// 	in al, dx		; grab a status byte
// 	test al, 0x80		; BSY flag set?
// 	jne short .retry
// 	test al, 8		; DRQ set?
// 	jne short .data_rdy
// .retry:
// 	dec ecx
// 	jg short .lp1
// ; need to wait some more -- loop until BSY clears or ERR sets (error exit if ERR sets)
//
// .pior_l:
// 	in al, dx		; grab a status byte
// 	test al, 0x80		; BSY flag set?
// 	jne short .pior_l	; (all other flags are meaningless if BSY is set)
// 	test al, 0x21		; ERR or DF set?
// 	jne short .fail
// .data_rdy:
// ; if BSY and ERR are clear then DRQ must be set -- go and read the data
// 	sub dl, 7		; read from data port (ie. 0x1f0)
// 	mov cx, 256
// 	rep insw		; gulp one 512b sector into edi
// 	or dl, 7		; "point" dx back at the status register
// 	in al, dx		; delay 400ns to allow drive to set new values of BSY and DRQ
// 	in al, dx
// 	in al, dx
// 	in al, dx
//
// ; After each DRQ data block it is mandatory to either:
// ; receive and ack the IRQ -- or poll the status port all over again
//
// 	inc ebp			; increment the current absolute LBA (overflowing is OK!)
// 	dec ebx			; decrement the "sectors to read" count
// 	test bx, bx		; check if "sectorcount" just decremented to 0
// 	jne short .pior_l
//
// 	sub dx, 7		; "point" dx back at the base IO port, so it's unchanged
// 	sub ebp, [esi + dd_stLBA]	; convert absolute lba back to relative
// ; this sub handles the >32bit overflow cases correcty, too
// ; "test" sets the zero flag for a "success" return -- also clears the carry flag
// 	test al, 0x21		; test the last status ERR bits
// 	je short .done
// .fail:
// 	stc
// .done:
// 	ret
//
//
// ; do a singletasking PIO ata "software reset" with DCR in dx
// srst_ata_st:
// 	push eax
// 	mov al, 4
// 	out dx, al			; do a "software reset" on the bus
// 	xor eax, eax
// 	out dx, al			; reset the bus to normal operation
// 	in al, dx			; it might take 4 tries for status bits to reset
// 	in al, dx			; ie. do a 400ns delay
// 	in al, dx
// 	in al, dx
// .rdylp:
// 	in al, dx
// 	and al, 0xc0			; check BSY and RDY
// 	cmp al, 0x40			; want BSY clear and RDY set
// 	jne short .rdylp
// 	pop eax
// 	ret

use crate::errors::Error;
use crate::println;
use lazy_static::lazy_static;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

lazy_static! {
    /// The ATA drives.
    pub static ref ATA_DRIVES: [spin::Mutex<Option<AtaDrive>>; 2] = [
        spin::Mutex::new(None),
        spin::Mutex::new(None),
    ];
}

/// Registers for the ATA drive.
///
/// The registers are used to communicate with the ATA drive.
///
/// # Variants
///
/// * `Data` - The data register.
/// * `Error` - The error register.
/// * `Features` - The features register.
/// * `SectorCount` - The sector count register.
/// * `SectorNumber` - The sector number register.
/// * `CylinderLow` - The cylinder low register.
/// * `CylinderHigh` - The cylinder high register.
/// * `DriveHead` - The drive head register.
/// * `Status` - The status register.
/// * `Command` - The command register.
/// * `AlternateStatus` - The alternate status register.
/// * `DeviceControl` - The device control register.
/// * `DriveAddress` - The drive address register.
#[derive(Debug)]
pub enum Register {
    Data(Port<u16>),
    Error(PortReadOnly<u8>),
    Features(PortWriteOnly<u8>),
    SectorCount(Port<u8>),
    SectorNumber(Port<u8>),
    CylinderLow(Port<u8>),
    CylinderHigh(Port<u8>),
    DriveHead(Port<u8>),
    Status(PortReadOnly<u8>),
    Command(PortWriteOnly<u8>),
    AlternateStatus(PortReadOnly<u8>),
    DeviceControl(PortWriteOnly<u8>),
    DriveAddress(PortReadOnly<u8>),
}

impl Register {
    /// Reads the value of the register.
    ///
    /// # Returns
    ///
    /// * `Result<u16, Error>` - The value of the register.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    pub fn read(&mut self) -> Result<u16, Error> {
        let value = unsafe {
            match self {
                Self::Data(port) => port.read(),

                Self::Error(port)
                | Self::Status(port)
                | Self::AlternateStatus(port)
                | Self::DriveAddress(port) => u16::from(port.read()),

                Self::SectorCount(port)
                | Self::SectorNumber(port)
                | Self::CylinderLow(port)
                | Self::CylinderHigh(port)
                | Self::DriveHead(port) => u16::from(port.read()),

                Self::Features(_) | Self::Command(_) | Self::DeviceControl(_) => {
                    return Err(Error::InvalidRegister("Register is write-only!".into()))
                }
            }
        };

        Ok(value)
    }

    /// Writes a value to the register.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to write to the register.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is read-only.
    pub fn write(&mut self, value: u16) -> Result<(), Error> {
        unsafe {
            match self {
                Self::Data(port) => port.write(value),

                Self::SectorCount(port)
                | Self::SectorNumber(port)
                | Self::CylinderLow(port)
                | Self::CylinderHigh(port)
                | Self::DriveHead(port) => port.write(value as u8),

                Self::Features(port) | Self::Command(port) | Self::DeviceControl(port) => {
                    port.write(value as u8)
                }

                Self::Error(_)
                | Self::Status(_)
                | Self::AlternateStatus(_)
                | Self::DriveAddress(_) => {
                    return Err(Error::InvalidRegister("Register is read-only!".into()))
                }
            }
        };

        Ok(())
    }
}

/// The ATA drive.
///
/// # Fields
///
/// * `data_register` - The data register.
/// * `error_register` - The error register.
/// * `features_register` - The features register.
/// * `sector_count_register` - The sector count register.
/// * `sector_number_register` - The sector number register.
/// * `cylinder_low_register` - The cylinder low register.
/// * `cylinder_high_register` - The cylinder high register.
/// * `drive_head_register` - The drive head register.
/// * `status_register` - The status register.
/// * `command_register` - The command register.
/// * `alternate_status_register` - The alternate status register.
/// * `device_control_register` - The device control register.
/// * `drive_address_register` - The drive address register.
#[derive(Debug)]
pub struct AtaDrive {
    data_register: Register,
    error_register: Register,
    features_register: Register,
    sector_count_register: Register,
    sector_number_register: Register,
    cylinder_low_register: Register,
    cylinder_high_register: Register,
    drive_head_register: Register,
    status_register: Register,
    command_register: Register,
    alternate_status_register: Register,
    device_control_register: Register,
    drive_address_register: Register,
}

impl AtaDrive {
    /// Creates a new ATA drive.
    ///
    /// # Arguments
    ///
    /// * `io_base` - The I/O base address of the ATA drive.
    /// * `cmd_base` - The command base address of the ATA drive.
    ///
    /// # Returns
    ///
    /// * `AtaDrive` - The new ATA drive.
    #[must_use]
    pub const fn new(io_base: u16, cmd_base: u16) -> Self {
        Self {
            data_register: Register::Data(Port::new(io_base)),
            error_register: Register::Error(PortReadOnly::new(io_base + 1)),
            features_register: Register::Features(PortWriteOnly::new(io_base + 1)),
            sector_count_register: Register::SectorCount(Port::new(io_base + 2)),
            sector_number_register: Register::SectorNumber(Port::new(io_base + 3)),
            cylinder_low_register: Register::CylinderLow(Port::new(io_base + 4)),
            cylinder_high_register: Register::CylinderHigh(Port::new(io_base + 5)),
            drive_head_register: Register::DriveHead(Port::new(io_base + 6)),
            status_register: Register::Status(PortReadOnly::new(io_base + 7)),
            command_register: Register::Command(PortWriteOnly::new(io_base + 7)),
            alternate_status_register: Register::AlternateStatus(PortReadOnly::new(cmd_base)),
            device_control_register: Register::DeviceControl(PortWriteOnly::new(cmd_base)),
            drive_address_register: Register::DriveAddress(PortReadOnly::new(cmd_base + 1)),
        }
    }

    /// Reads a sector from the ATA drive.
    ///
    /// # Arguments
    ///
    /// * `lba` - The logical block address of the sector to read.
    /// * `buffer` - The buffer to read the sector into.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    pub fn read_sector(&mut self, lba: u64, buffer: &mut [u8]) -> Result<(), Error> {
        self.read_sector_internal(lba, buffer)?;

        Ok(())
    }

    /// Writes a sector to the ATA drive.
    ///
    /// # Arguments
    ///
    /// * `lba` - The logical block address of the sector to write.
    /// * `buffer` - The buffer to write the sector from.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    pub fn write_sector(&mut self, lba: u64, buffer: &[u8]) -> Result<(), Error> {
        let sector_count = buffer.len() / 512;
        let mut sector = 0;

        while sector < sector_count {
            self.wait_for_not_busy()?;
            self.wait_for_ready()?;
            self.select_drive(lba + sector as u64)?;
            self.wait_for_not_busy()?;
            self.wait_for_ready()?;
            self.write_sector_internal(lba + sector as u64, &buffer[sector * 512..])?;

            sector += 1;
        }

        Ok(())
    }

    /// Waits for the drive to be ready.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    fn wait_for_ready(&mut self) -> Result<(), Error> {
        let mut status = self.alternate_status_register.read()?;

        while status & 0x80 != 0x80 {
            status = self.alternate_status_register.read()?;
        }

        Ok(())
    }

    /// Waits for the drive to not be busy.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    fn wait_for_not_busy(&mut self) -> Result<(), Error> {
        let mut status = self.alternate_status_register.read()?;

        while status & 0x80 == 0x80 {
            status = self.alternate_status_register.read()?;
        }

        Ok(())
    }

    /// Selects the drive.
    ///
    /// # Arguments
    ///
    /// * `lba` - The logical block address of the sector to select.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    /// * `Error::InvalidRegister` - The register is read-only.
    fn select_drive(&mut self, lba: u64) -> Result<(), Error> {
        let mut drive_head = self.drive_head_register.read()?;
        drive_head &= 0xF0;
        drive_head |= ((lba >> 24) & 0x0F) as u16;

        self.drive_head_register.write(drive_head)?;

        Ok(())
    }

    /// Reads a sector from the ATA drive.
    ///
    /// # Arguments
    ///
    /// * `lba` - The logical block address of the sector to read.
    /// * `buffer` - The buffer to read the sector into.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    /// * `Error::InvalidRegister` - The register is read-only.
    fn read_sector_internal(&mut self, lba: u64, buffer: &mut [u8]) -> Result<(), Error> {
        let sector_count = buffer.len() / 512;
        let mut sector = 0;

        while sector < sector_count {
            self.wait_for_not_busy()?;
            self.wait_for_ready()?;
            self.select_drive(lba + sector as u64)?;
            self.wait_for_not_busy()?;
            self.wait_for_ready()?;
            self.read_sector_internal(lba + sector as u64, &mut buffer[sector * 512..])?;

            sector += 1;
        }

        Ok(())
    }

    /// Writes a sector to the ATA drive.
    ///
    /// # Arguments
    ///
    /// * `lba` - The logical block address of the sector to write.
    /// * `buffer` - The buffer to write the sector from.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidRegister` - The register is write-only.
    /// * `Error::InvalidRegister` - The register is read-only.
    fn write_sector_internal(&mut self, lba: u64, buffer: &[u8]) -> Result<(), Error> {
        let sector_count = buffer.len() / 512;
        let mut sector = 0;

        while sector < sector_count {
            self.wait_for_not_busy()?;
            self.wait_for_ready()?;
            self.select_drive(lba + sector as u64)?;
            self.wait_for_not_busy()?;
            self.wait_for_ready()?;
            self.write_sector_internal(lba + sector as u64, &buffer[sector * 512..])?;

            sector += 1;
        }

        Ok(())
    }
}

/// Initializes the ATA driver.
pub fn init() {
    let mut drive = AtaDrive::new(0x1F0, 0x3F6);

    println!("[INFO]: ATA driver initialized!");
}
