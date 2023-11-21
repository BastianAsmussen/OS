/*
use crate::dev::ata;
use crate::errors::Error;
use alloc::vec;
use alloc::vec::Vec;

/// The size of a FAT entry.
///
/// FAT12 uses 12 bits per entry, so 2 bytes are needed.
const FAT_ENTRY_SIZE: usize = 2;

/// FAT12 file system.
///
/// # Fields
///
/// * `bytes_per_sector` - The number of bytes per sector.
/// * `sectors_per_cluster` - The number of sectors per cluster.
/// * `reserved_sectors` - The number of reserved sectors.
/// * `fat_count` - The number of FATs.
/// * `root_dir_entries` - The number of root directory entries.
/// * `total_sectors` - The total number of sectors.
/// * `root_dir_sectors` - The number of root directory sectors.
#[derive(Debug)]
pub struct Fat12 {
    bytes_per_block: u16,
    blocks_per_cluster: u8,
    reserved_blocks: u16,
    fat_count: u8,
    root_dir_entries: u16,
    total_blocks: u16,
    root_dir_blocks: u16,
}

impl Fat12 {
    /// Creates a new FAT12 file system.
    ///
    /// # Arguments
    ///
    /// * `bytes_per_block` - The number of bytes per block.
    /// * `blocks_per_cluster` - The number of sectors per cluster.
    /// * `reserved_blocks` - The number of reserved blocks.
    /// * `fat_count` - The number of FATs.
    /// * `root_dir_entries` - The number of root directory entries.
    /// * `total_blocks` - The total number of blocks.
    /// * `root_dir_blocks` - The number of root directory blocks.
    ///
    /// # Returns
    ///
    /// * `Self` - The new FAT12 file system.
    #[must_use]
    pub const fn new(
        bytes_per_block: u16,
        blocks_per_cluster: u8,
        reserved_blocks: u16,
        fat_count: u8,
        root_dir_entries: u16,
        total_blocks: u16,
        root_dir_blocks: u16,
    ) -> Self {
        Self {
            bytes_per_block,
            blocks_per_cluster,
            reserved_blocks,
            fat_count,
            root_dir_entries,
            total_blocks,
            root_dir_blocks,
        }
    }

    /// Reads a block from the disk.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus number.
    /// * `drive` - The drive number.
    /// * `block` - The block number.
    /// * `buffer` - The buffer to read into.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the block cannot be read.
    pub fn read_block(
        &self,
        bus: u8,
        drive: u8,
        block: u32,
        buffer: &mut [u8],
    ) -> Result<(), Error> {
        ata::read(bus, drive, block, buffer)
    }

    /// Reads a FAT entry.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus number.
    /// * `drive` - The drive number.
    /// * `cluster` - The cluster number.
    /// * `fat_buffer` - The buffer to read into.
    ///
    /// # Returns
    ///
    /// * `Result<u16, Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the FAT entry cannot be read.
    fn read_fat_entry(
        &self,
        bus: u8,
        drive: u8,
        cluster: u16,
        fat_buffer: &mut [u8],
    ) -> Result<u16, Error> {
        let fat_offset = u32::from(self.reserved_blocks) * u32::from(self.bytes_per_block);
        let fat_entry_offset = fat_offset + (u32::from(cluster) * u32::try_from(FAT_ENTRY_SIZE)?);
        self.read_block(bus, drive, fat_entry_offset, fat_buffer)?;

        let cluster_bytes = u16::from_le_bytes(fat_buffer.try_into()?);

        Ok(if cluster % 2 == 0 {
            cluster_bytes & 0xFFF
        } else {
            cluster_bytes >> 4
        })
    }

    /// Reads a file from the disk.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus number.
    /// * `drive` - The drive number.
    /// * `start_cluster` - The start cluster.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the file cannot be read.
    pub fn read_file(&self, bus: u8, drive: u8, start_cluster: u16) -> Result<Vec<u8>, Error> {
        let mut cluster = start_cluster;
        let mut file_data = Vec::new();
        let mut fat_buffer = [0u8; FAT_ENTRY_SIZE];

        loop {
            let block_offset = u32::from(self.reserved_blocks)
                + u32::from(self.root_dir_blocks)
                + u32::from(cluster - 2) * u32::from(self.blocks_per_cluster);

            for i in 0..self.blocks_per_cluster {
                let block = block_offset + u32::from(i);
                let mut buffer = vec![0u8; self.bytes_per_block as usize];
                self.read_block(bus, drive, block, &mut buffer)?;

                file_data.extend_from_slice(&buffer);
            }

            cluster = self.read_fat_entry(bus, drive, cluster, &mut fat_buffer)?;
            if cluster >= 0xFF8 {
                break;
            }
        }

        Ok(file_data)
    }

    /// Writes a block to the disk.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus number.
    /// * `drive` - The drive number.
    /// * `block` - The block number.
    /// * `buffer` - The buffer to write from.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the block cannot be written.
    pub fn write_block(&self, bus: u8, drive: u8, block: u32, buffer: &[u8]) -> Result<(), Error> {
        ata::write(bus, drive, block, buffer)
    }

    /// Writes a FAT entry.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus number.
    /// * `drive` - The drive number.
    /// * `cluster` - The cluster number.
    /// * `value` - The value to write.
    /// * `fat_buffer` - The buffer to write from.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the FAT entry cannot be written.
    fn write_fat_entry(
        &self,
        bus: u8,
        drive: u8,
        cluster: u16,
        value: u16,
        fat_buffer: &mut [u8],
    ) -> Result<(), Error> {
        let fat_offset = u32::from(self.reserved_blocks) * u32::from(self.bytes_per_block);
        let fat_entry_offset = fat_offset + (u32::from(cluster) * u32::try_from(FAT_ENTRY_SIZE)?);
        fat_buffer[..2].copy_from_slice(&value.to_le_bytes());
        self.write_block(bus, drive, fat_entry_offset, fat_buffer)
    }

    /// Writes a file to the disk.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus number.
    /// * `drive` - The drive number.
    /// * `start_cluster` - The start cluster.
    /// * `data` - The data to write.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - The result of the operation.
    ///
    /// # Errors
    ///
    /// * If the file cannot be written.
    pub fn write_file(
        &self,
        bus: u8,
        drive: u8,
        start_cluster: u16,
        data: &[u8],
    ) -> Result<(), Error> {
        let mut cluster = start_cluster;
        let mut fat_buffer = [0u8; FAT_ENTRY_SIZE];

        // Find the last cluster of the file
        while self.read_fat_entry(bus, drive, cluster, &mut fat_buffer)? < 0xFF8 {
            cluster = self.read_fat_entry(bus, drive, cluster, &mut fat_buffer)?;
        }

        // Write the data to the disk
        let mut remaining_data = data;
        while !remaining_data.is_empty() {
            let block_offset = u32::from(self.reserved_blocks)
                + u32::from(self.root_dir_blocks)
                + u32::from(cluster - 2) * u32::from(self.blocks_per_cluster);

            for i in 0..self.blocks_per_cluster {
                let block = block_offset + u32::from(i);
                let write_size =
                    core::cmp::min(remaining_data.len(), self.bytes_per_block as usize);
                self.write_block(bus, drive, block, &remaining_data[..write_size])?;
                remaining_data = &remaining_data[write_size..];

                if remaining_data.is_empty() {
                    break;
                }
            }

            if !remaining_data.is_empty() {
                // Allocate a new cluster in the FAT
                let new_cluster = self.find_free_cluster(bus, drive)?;
                self.write_fat_entry(bus, drive, cluster, new_cluster, &mut fat_buffer)?;

                // Update current cluster to the new one
                cluster = new_cluster;
            }
        }

        // Mark the last cluster as end of file
        self.write_fat_entry(bus, drive, cluster, 0xFFF, &mut fat_buffer)?;

        Ok(())
    }

    /// Finds a free cluster.
    ///
    /// # Arguments
    ///
    /// * `bus` - The bus number.
    /// * `drive` - The drive number.
    ///
    /// # Returns
    ///
    /// * `Result<u16, Error>` - The result of the operation, containing the free cluster's number.
    ///
    /// # Errors
    ///
    /// * If no free cluster is found.
    fn find_free_cluster(&self, bus: u8, drive: u8) -> Result<u16, Error> {
        let mut fat_buffer = [0u8; FAT_ENTRY_SIZE];
        for cluster in 2..=0xFF7 {
            if self.read_fat_entry(bus, drive, cluster, &mut fat_buffer)? == 0 {
                return Ok(cluster);
            }
        }

        Err(Error::FileSystem("No free cluster found!".into()))
    }
}

/// Initializes the FAT12 file system.
///
/// # Returns
///
/// * `Result<Vec<Fat12>, Error>` - The result of the operation.
///
/// # Errors
///
/// * If the file system cannot be initialized.
pub fn init() -> Result<Vec<Fat12>, Error> {
    let mut file_systems = Vec::new();

    for drive in ata::list_drives() {
        let mut buffer = [0u8; 512];
        ata::read(drive.bus, drive.disk, 0, &mut buffer)?;

        let bytes_per_block = u16::from_le_bytes(buffer[11..13].try_into()?);
        let blocks_per_cluster = buffer[13];
        let reserved_blocks = u16::from_le_bytes(buffer[14..16].try_into()?);
        let fat_count = buffer[16];
        let root_dir_entries = u16::from_le_bytes(buffer[17..19].try_into()?);
        let total_blocks = u16::from_le_bytes(buffer[19..21].try_into()?);
        let root_dir_blocks = ((root_dir_entries * 32) + (bytes_per_block - 1)) / bytes_per_block;

        let file_system = Fat12::new(
            bytes_per_block,
            blocks_per_cluster,
            reserved_blocks,
            fat_count,
            root_dir_entries,
            total_blocks,
            root_dir_blocks,
        );

        file_systems.push(file_system);
    }

    Ok(file_systems)
}
 */
