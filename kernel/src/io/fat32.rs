use alloc::string::{String, ToString};
use alloc::{format, vec};
use alloc::vec::Vec;
use crate::io::{File, FileKind, FileSystem};
use crate::io::ata::{ata_read_sector, ata_read_sectors};

pub struct Fat32FileSystem {
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    sectors_per_fat: u32,
    root_cluster: u32,
    fat_start_lba: u32,
    data_start_lba: u32,
}

impl Fat32FileSystem {
    pub fn new() -> Self {
        let mut sector = [0u8; 512];
        ata_read_sector(0, &mut sector);

        let bytes_per_sector = u16::from_le_bytes([sector[11], sector[12]]);
        let sectors_per_cluster = sector[13];
        let reserved_sectors = u16::from_le_bytes([sector[14], sector[15]]);
        let fat_count = sector[16];
        let sectors_per_fat = u32::from_le_bytes([sector[36], sector[37], sector[38], sector[39]]);
        let root_cluster = u32::from_le_bytes([sector[44], sector[45], sector[46], sector[47]]);

        let fat_start_lba = reserved_sectors as u32;
        let data_start_lba = fat_start_lba + (fat_count as u32 * sectors_per_fat);

        Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            fat_count,
            sectors_per_fat,
            root_cluster,
            fat_start_lba,
            data_start_lba,
        }
    }

    fn cluster_to_lba(&self, cluster: u32) -> u32 {
        self.data_start_lba + (cluster - 2) * self.sectors_per_cluster as u32
    }

    fn read_cluster(&self, cluster: u32) -> Vec<u8> {
        let lba = self.cluster_to_lba(cluster);
        let mut buf = vec![0u8; self.bytes_per_sector as usize * self.sectors_per_cluster as usize];
        ata_read_sectors(lba, &mut buf);
        buf
    }

    fn read_fat_entry(&self, cluster: u32) -> u32 {
        let fat_offset = cluster * 4;
        let fat_sector = self.fat_start_lba + (fat_offset / 512);
        let offset_in_sector = (fat_offset % 512) as usize;

        let mut buf = [0u8; 512];
        ata_read_sector(fat_sector, &mut buf);
        u32::from_le_bytes([
            buf[offset_in_sector],
            buf[offset_in_sector + 1],
            buf[offset_in_sector + 2],
            buf[offset_in_sector + 3],
        ]) & 0x0FFFFFFF
    }

    fn read_cluster_chain(&self, start_cluster: u32) -> Vec<u8> {
        let mut cluster = start_cluster;
        let mut data = Vec::new();

        loop {
            let cluster_data = self.read_cluster(cluster);
            data.extend_from_slice(&cluster_data);

            let next = self.read_fat_entry(cluster);
            if next >= 0xFFFFFF8 {
                break;
            }
            cluster = next;
        }

        data
    }
}

impl FileSystem for Fat32FileSystem {
    fn open(&self, path: &str) -> Option<File> {
        let mut current = self.root();
        if path == "/" {
            return Some(current);
        }

        let parts = path.trim_start_matches('/').split('/');

        for part in parts {
            let children = self.list_dir(&current);
            let next = children.into_iter().find(|f| f.filename().eq_ignore_ascii_case(part));
            if let Some(found) = next {
                current = found;
            } else {
                return None;
            }
        }

        Some(current)
    }

    fn read_file<'a>(&self, file: &'a mut File) -> Option<&'a [u8]> {
        if file.kind != FileKind::File {
            return None;
        }

        if file.data.is_none() {
            let buf = self.read_cluster_chain(file.start_cluster);
            file.data = Some(buf);
        }

        file.data.as_deref()
    }

    fn list_dir(&self, dir: &File) -> Vec<File> {
        if dir.kind != FileKind::Directory {
            return vec![];
        }

        let raw_data = self.read_cluster_chain(dir.start_cluster);
        let mut files = Vec::new();

        let mut lfn_entries: Vec<&[u8]> = Vec::new();

        let mut i = 0;
        while i + 32 <= raw_data.len() {
            let entry = &raw_data[i..i + 32];
            i += 32;

            if entry[0] == 0x00 {
                break;
            }
            if entry[11] == 0xE5 {
                continue;
            }
            if entry[11] == 0x0F {
                lfn_entries.push(entry);
                continue;
            }

            let name = if !lfn_entries.is_empty() {
                let name = read_lfn_name(&lfn_entries).unwrap_or_else(|| "INVALID_LFN".into());
                lfn_entries.clear();
                name
            } else {
                let name = core::str::from_utf8(&entry[0..8]).unwrap_or("").trim();
                let ext = core::str::from_utf8(&entry[8..11]).unwrap_or("").trim();
                if ext.is_empty() {
                    name.to_string()
                } else {
                    format!("{}.{}", name, ext)
                }
            };

            let attr = entry[11];
            let kind = if attr & 0x10 != 0 {
                FileKind::Directory
            } else {
                FileKind::File
            };

            let cluster_lo = u16::from_le_bytes([entry[26], entry[27]]) as u32;
            let cluster_hi = u16::from_le_bytes([entry[20], entry[21]]) as u32;
            let start_cluster = (cluster_hi << 16) | cluster_lo;

            let size = u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]);

            files.push(File {
                path: name,
                kind,
                start_cluster,
                size,
                data: None,
                fs: self
            });
        }

        files
    }

    fn root(&self) -> File {
        File {
            kind: FileKind::Directory,
            path: "/".to_string(),
            start_cluster: self.root_cluster,
            size: 0,
            data: None,
            fs: self
        }
    }
}

fn read_lfn_name(lfn_stack: &[&[u8]]) -> Option<String> {
    let mut name_utf16: Vec<u16> = Vec::new();
    for entry in lfn_stack.iter().rev() {
        for &range in &[
            (1, 10),
            (14, 12),
            (28, 4)
        ] {
            for chunk in entry[range.0..range.0 + range.1].chunks(2) {
                let code = u16::from_le_bytes([chunk[0], chunk[1]]);
                if code == 0x0000 || code == 0xFFFF {
                    continue;
                }
                name_utf16.push(code);
            }
        }
    }

    String::from_utf16(&name_utf16).ok()
}