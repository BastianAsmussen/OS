use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Specifies the file is read only.
pub const READ_ONLY: u8 = 0x01;
/// Specifies the file is hidden.
pub const HIDDEN: u8 = 0x02;
/// Specifies the file is a system file.
///
/// # Notes
/// * System files are files that are critical to the operation of the operating system.
pub const SYSTEM: u8 = 0x04;
/// Specifies the file is a volume ID.
pub const VOLUME_ID: u8 = 0x08;
/// Specifies the file is a directory.
pub const DIRECTORY: u8 = 0x10;
/// Specifies the file is an archive.
///
/// # Notes
/// * Archive files are files that are marked for backup or removal.
pub const ARCHIVE: u8 = 0x20;
/// Specifies the file is a long file name.
///
/// # Notes
///
/// * Long file names are files that have a name longer than 8 characters.
/// * They're defined by having the `READ_ONLY`, `HIDDEN`, `SYSTEM`, or `VOLUME_ID` flags set.
pub const LFN: u8 = READ_ONLY | HIDDEN | SYSTEM | VOLUME_ID;

/// A FAT file system.
///
/// # Fields
///
/// * `boot_sector` - The boot sector.
/// * `fat` - The file allocation table.
/// * `root_dir` - The root directory.
#[derive(Debug, Clone)]
pub struct Fat {
    boot_sector: BootSector,
    fat: FatTable,
    root_dir: RootDirectory,
}

impl Fat {
    /// Creates a new FAT file system.
    ///
    /// # Arguments
    ///
    /// * `boot_sector` - The boot sector.
    /// * `fat` - The file allocation table.
    /// * `root_dir` - The root directory.
    ///
    /// # Returns
    ///
    /// * The new FAT file system.
    #[must_use]
    pub const fn new(boot_sector: BootSector, fat: FatTable, root_dir: RootDirectory) -> Self {
        Self {
            boot_sector,
            fat,
            root_dir,
        }
    }

    /// Reads a file from the file system.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    ///
    /// # Returns
    ///
    /// * If the file exists, the file.
    /// * Otherwise, `None`.
    #[must_use]
    pub fn read_file(&self, path: &str) -> Option<File> {
        // Get the file name.
        let file_name = path.split('/').last()?;

        // Get the directory.
        let dir = path.trim_end_matches(file_name);

        // Get the directory entry.
        let dir_entry = self.root_dir.get_entry(dir)?;

        // Get the file entry.
        let file_entry = dir_entry.get_entry(file_name)?;

        // Get the file.
        let file = self.root_dir.get_file(&file_entry)?;

        // Return the file.
        Some(file)
    }

    /// Reads a directory from the file system.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the directory.
    ///
    /// # Returns
    ///
    /// * If the directory exists, the directory.
    /// * Otherwise, `None`.
    #[must_use]
    pub fn read_dir(&self, path: &str) -> Option<Vec<File>> {
        // Get the directory name.
        let dir_name = path.split('/').last()?;

        // Get the directory.
        let dir = path.trim_end_matches(dir_name);

        // Get the directory entry.
        let dir_entry = self.root_dir.get_entry(dir)?;

        // Get the directory entry.
        let dir_entry = dir_entry.get_entry(dir_name)?;

        // Check if the directory entry is a directory.
        if dir_entry.attributes & DIRECTORY == 0 {
            // Return `None`.
            return None;
        }

        // Get the first cluster.
        let first_cluster = dir_entry.first_cluster;

        // Get the files.
        let files = self.get_files(first_cluster)?;

        // Return the files.
        Some(files)
    }

    /// Gets the files in the specified cluster.
    ///
    /// # Arguments
    ///
    /// * `cluster` - The cluster.
    ///
    /// # Returns
    ///
    /// * If the cluster exists, the files.
    /// * Otherwise, `None`.
    #[must_use]
    pub fn get_files(&self, cluster: u32) -> Option<Vec<File>> {
        // Get the first cluster.
        let mut cluster = cluster;

        // Create the files vector.
        let mut files = Vec::new();

        // Loop until the cluster is `None`.
        loop {
            // Get the file entry.
            let file_entry = self.get_file_entry(cluster)?;

            // Get the file.
            let file = self.root_dir.get_file(&file_entry)?;

            // Add the file to the files vector.
            files.push(file);

            // Get the next cluster.
            let next_cluster = match self.fat.next_cluster(cluster) {
                Some(next_cluster) => Some(next_cluster),
                None => {
                    // Return the files.
                    return Some(files);
                }
            };

            // Set the cluster to the next cluster.
            cluster = next_cluster?;
        }
    }

    /// Gets the file entry for the specified cluster.
    ///
    /// # Arguments
    ///
    /// * `cluster` - The cluster.
    ///
    /// # Returns
    ///
    /// * If the cluster exists, the file entry.
    /// * Otherwise, `None`.
    #[must_use]
    pub const fn get_file_entry(&self, cluster: u32) -> Option<DirectoryEntry> {
        // Get the sector.
        let sector = self.boot_sector.reserved_sectors as u32
            + self.boot_sector.fat_count as u32 * self.boot_sector.sectors_per_fat as u32
            + (cluster - 2) * self.boot_sector.sectors_per_cluster as u32;

        // Get the sector.
        let sector = sector as usize;

        // Get the sector.
        let sector = unsafe { &*(sector as *const [u8; 512]) };

        // Get the file entry.
        let file_entry = DirectoryEntry::new(
            "",
            DIRECTORY,
            [0; 10],
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            sector[0x1A] as u16 | ((sector[0x1B] as u16) << 8),
            sector[0x1C] as u32
                | ((sector[0x1D] as u32) << 8)
                | ((sector[0x1E] as u32) << 16)
                | ((sector[0x1F] as u32) << 24),
            cluster,
        );

        // Return the file entry.
        Some(file_entry)
    }

    /// Gets the file entry for the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path.
    ///
    /// # Returns
    ///
    /// * If the file entry exists, the file entry.
    /// * Otherwise, `None`.
    #[must_use]
    pub fn get_file_entry_from_path(&self, path: &str) -> Option<DirectoryEntry> {
        // Get the file name.
        let file_name = path.split('/').last()?;

        // Get the directory.
        let dir = path.trim_end_matches(file_name);

        // Get the directory entry.
        let dir_entry = self.root_dir.get_entry(dir)?;

        // Get the file entry.
        let file_entry = dir_entry.get_entry(file_name)?;

        // Return the file entry.
        Some(file_entry)
    }
}

/// A FAT file system boot sector.
///
/// # Fields
///
/// * `bytes_per_sector` - The number of bytes per sector.
/// * `sectors_per_cluster` - The number of sectors per cluster.
/// * `reserved_sectors` - The number of reserved sectors.
/// * `fat_count` - The number of FAT tables.
/// * `root_dir_entries` - The number of root directory entries.
/// * `total_sectors` - The total number of sectors.
/// * `sectors_per_fat` - The number of sectors per FAT.
/// * `sectors_per_track` - The number of sectors per track.
/// * `head_count` - The number of heads.
/// * `hidden_sectors` - The number of hidden sectors.
/// * `total_sectors_long` - The total number of sectors.
#[derive(Debug, Clone, Copy)]
pub struct BootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_dir_entries: u16,
    pub total_sectors: u16,
    pub sectors_per_fat: u16,
    pub sectors_per_track: u16,
    pub head_count: u16,
    pub hidden_sectors: u32,
    pub total_sectors_long: u32,
}

impl BootSector {
    /// Creates a new FAT file system boot sector.
    ///
    /// # Arguments
    ///
    /// * `bytes_per_sector` - The number of bytes per sector.
    /// * `sectors_per_cluster` - The number of sectors per cluster.
    /// * `reserved_sectors` - The number of reserved sectors.
    /// * `fat_count` - The number of FAT tables.
    /// * `root_dir_entries` - The number of root directory entries.
    /// * `total_sectors` - The total number of sectors.
    /// * `sectors_per_fat` - The number of sectors per FAT.
    /// * `sectors_per_track` - The number of sectors per track.
    /// * `head_count` - The number of heads.
    /// * `hidden_sectors` - The number of hidden sectors.
    /// * `total_sectors_long` - The total number of sectors.
    ///
    /// # Returns
    ///
    /// * The new FAT file system boot sector.
    #[must_use]
    pub const fn new(
        bytes_per_sector: u16,
        sectors_per_cluster: u8,
        reserved_sectors: u16,
        fat_count: u8,
        root_dir_entries: u16,
        total_sectors: u16,
        sectors_per_fat: u16,
        sectors_per_track: u16,
        head_count: u16,
        hidden_sectors: u32,
        total_sectors_long: u32,
    ) -> Self {
        Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            fat_count,
            root_dir_entries,
            total_sectors,
            sectors_per_fat,
            sectors_per_track,
            head_count,
            hidden_sectors,
            total_sectors_long,
        }
    }
}

/// A FAT file system file allocation table.
///
/// # Fields
///
/// * `entries` - The entries.
#[derive(Debug, Clone, Copy)]
pub struct FatTable {
    entries: [u32; 128],
}

impl FatTable {
    /// Creates a new FAT file system file allocation table.
    ///
    /// # Arguments
    ///
    /// * `entries` - The entries.
    ///
    /// # Returns
    ///
    /// * The new FAT file system file allocation table.
    #[must_use]
    pub const fn new(entries: [u32; 128]) -> Self {
        Self { entries }
    }

    /// Gets the next cluster in the chain.
    ///
    /// # Arguments
    ///
    /// * `cluster` - The current cluster.
    ///
    /// # Returns
    ///
    /// * The next cluster in the chain.
    #[must_use]
    pub const fn next_cluster(&self, cluster: u32) -> Option<u32> {
        // Get the entry.
        let entry = self.entries[cluster as usize];

        // Check if the entry is valid.
        if entry >= 0x0FFF_FFF8 {
            // Return `None`.
            return None;
        }

        // Return the entry.
        Some(entry)
    }
}

/// A FAT file system root directory.
///
/// # Fields
///
/// * `entries` - The entries.
#[derive(Debug, Clone)]
pub struct RootDirectory {
    entries: [DirectoryEntry; 512],
}

impl RootDirectory {
    /// Creates a new FAT file system root directory.
    ///
    /// # Arguments
    ///
    /// * `entries` - The entries.
    ///
    /// # Returns
    ///
    /// * The new FAT file system root directory.
    #[must_use]
    pub const fn new(entries: [DirectoryEntry; 512]) -> Self {
        Self { entries }
    }

    /// Gets the directory entry for the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path.
    ///
    /// # Returns
    ///
    /// * If the directory entry exists, the directory entry.
    /// * Otherwise, `None`.
    #[must_use]
    pub fn get_entry(&self, path: &str) -> Option<DirectoryEntry> {
        // Check if the path is empty.
        if path.is_empty() {
            // Return the root directory.
            return Some(DirectoryEntry::new(
                "",
                DIRECTORY,
                [0; 10],
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ));
        }

        // Get the directory name.
        let dir_name = path.split('/').last()?;

        // Get the directory.
        let dir = path.trim_end_matches(dir_name);

        // Get the directory entry.
        let dir_entry = self.get_entry(dir)?;

        // Get the directory entry.
        let dir_entry = dir_entry.get_entry(dir_name)?;

        // Return the directory entry.
        Some(dir_entry)
    }

    /// Gets the file for the specified file entry.
    ///
    /// # Arguments
    ///
    /// * `file_entry` - The file entry.
    ///
    /// # Returns
    ///
    /// * If the file exists, the file.
    /// * Otherwise, `None`.
    #[must_use]
    pub fn get_file(&self, file_entry: &DirectoryEntry) -> Option<File> {
        // Check if the file is a directory.
        if file_entry.attributes & DIRECTORY != 0 {
            // Return `None`.
            return None;
        }

        // Get the file name.
        let file_name = file_entry.name.trim_end_matches(' ');

        // Get the file size.
        let file_size = file_entry.file_size;

        // Get the first cluster.
        let first_cluster = file_entry.first_cluster;

        // Return the file.
        Some(File::new(file_name, file_size, first_cluster))
    }
}

/// A FAT file system directory entry.
///
/// # Fields
///
/// * `name` - The name.
/// * `attributes` - The attributes.
/// * `reserved` - The reserved bytes.
/// * `creation_time_tenths` - The creation time tenths of a second.
/// * `creation_time` - The creation time.
/// * `creation_date` - The creation date.
/// * `last_accessed` - The last accessed date.
/// * `first_cluster_high` - The high 16 bits of the first cluster.
/// * `last_modified_time` - The last modified time.
/// * `last_modified_date` - The last modified date.
/// * `first_cluster_low` - The low 16 bits of the first cluster.
/// * `file_size` - The file size.
/// * `first_cluster` - The first cluster.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectoryEntry {
    pub name: &'static str,
    pub attributes: u8,
    pub reserved: [u8; 10],
    pub creation_time_tenths: u8,
    pub creation_time: u16,
    pub creation_date: u16,
    pub last_accessed: u16,
    pub first_cluster_high: u16,
    pub last_modified_time: u16,
    pub last_modified_date: u16,
    pub first_cluster_low: u16,
    pub file_size: u32,
    pub first_cluster: u32,
}

impl DirectoryEntry {
    /// Creates a new FAT file system directory entry.
    ///
    /// # Arguments
    ///
    /// * `name` - The name.
    /// * `attributes` - The attributes.
    /// * `reserved` - The reserved bytes.
    /// * `creation_time_tenths` - The creation time tenths of a second.
    /// * `creation_time` - The creation time.
    /// * `creation_date` - The creation date.
    /// * `last_accessed` - The last accessed date.
    /// * `first_cluster_high` - The high 16 bits of the first cluster.
    /// * `last_modified_time` - The last modified time.
    /// * `last_modified_date` - The last modified date.
    /// * `first_cluster_low` - The low 16 bits of the first cluster.
    /// * `file_size` - The file size.
    /// * `first_cluster` - The first cluster.
    ///
    /// # Returns
    ///
    /// * The new FAT file system directory entry.
    #[must_use]
    pub const fn new(
        name: &'static str,
        attributes: u8,
        reserved: [u8; 10],
        creation_time_tenths: u8,
        creation_time: u16,
        creation_date: u16,
        last_accessed: u16,
        first_cluster_high: u16,
        last_modified_time: u16,
        last_modified_date: u16,
        first_cluster_low: u16,
        file_size: u32,
        first_cluster: u32,
    ) -> Self {
        Self {
            name,
            attributes,
            reserved,
            creation_time_tenths,
            creation_time,
            creation_date,
            last_accessed,
            first_cluster_high,
            last_modified_time,
            last_modified_date,
            first_cluster_low,
            file_size,
            first_cluster,
        }
    }

    /// Gets the directory entry for the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path.
    ///
    /// # Returns
    ///
    /// * If the directory entry exists, the directory entry
    /// * Otherwise, `None`.
    #[must_use]
    pub fn get_entry(&self, path: &str) -> Option<Self> {
        // Check if the path is empty.
        if path.is_empty() {
            // Return the directory entry.
            return Some(*self);
        }

        // Get the directory name.
        let dir_name = path.split('/').last()?;

        // Get the directory.
        let dir = path.trim_end_matches(dir_name);

        // Check if the directory name is `.`.
        if dir_name == "." {
            // Return the directory entry.
            return Some(*self);
        }

        // Check if the directory name is `..`.
        if dir_name == ".." {
            // Return the directory entry.
            return Some(*self);
        }

        // Check if the directory name is `LFN`.
        if self.attributes & LFN != 0 {
            // Return `None`.
            return None;
        }

        // Return `None`.
        None
    }
}

/// A FAT file system file.
///
/// # Fields
///
/// * `name` - The name.
/// * `size` - The size.
/// * `first_cluster` - The first cluster.
#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub size: u32,
    pub first_cluster: u32,
}

impl File {
    /// Creates a new FAT file system file.
    ///
    /// # Arguments
    ///
    /// * `name` - The name.
    /// * `size` - The size.
    /// * `first_cluster` - The first cluster.
    ///
    /// # Returns
    ///
    /// * The new FAT file system file.
    #[must_use]
    pub fn new(name: &str, size: u32, first_cluster: u32) -> Self {
        Self {
            name: name.to_string(),
            size,
            first_cluster,
        }
    }
}

/// Initializes the FAT file system.
///
/// # Returns
///
/// * The FAT file system.
#[must_use]
pub fn init() -> Fat {
    // Get the boot sector.
    let boot_sector = BootSector::new(
        512,
        1,
        1,
        2,
        512,
        0,
        0,
        0,
        0,
        0,
        0,
    );

    // Get the FAT table.
    let fat = FatTable::new([0; 128]);

    // Get the root directory.
    let root_dir = RootDirectory::new([DirectoryEntry::default(); 512]);

    // Return the FAT file system.
    Fat::new(boot_sector, fat, root_dir)
}
