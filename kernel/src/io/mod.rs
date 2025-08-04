use alloc::string::String;
use alloc::vec::Vec;

mod ata;
mod fat32;

pub use fat32::Fat32FileSystem;

#[derive(Eq, PartialEq)]
enum FileKind {
    File,
    Directory,
}

pub struct File<'a> {
    kind: FileKind,
    path: String,
    size: u32,
    data: Option<Vec<u8>>,
    fs: &'a dyn FileSystem,
    start_cluster: u32,
}

impl<'a> File<'a> {
    pub fn open<'fs: 'a>(fs: &'fs dyn FileSystem, path: &str) -> Option<Self> {
        fs.open(path)
    }

    pub fn read(&mut self) -> &[u8] {
        if self.data.is_none() {
            self.fs.read_file(self).expect("failed to read file");
        }

        self.data.as_ref().unwrap()
    }

    pub fn list_children(&self) -> Vec<File> {
        self.fs.list_dir(self)
    }

    pub fn is_directory(&self) -> bool {
        self.kind == FileKind::Directory
    }

    pub fn filename(&self) -> &str {
        let trimmed = self.path.trim_end_matches('/');
        trimmed.rsplit('/').next().unwrap_or("")
    }
}

pub trait FileSystem {
    fn open(&self, path: &str) -> Option<File>;
    fn read_file<'a>(&self, file: &'a mut File) -> Option<&'a [u8]>;
    fn list_dir(&self, dir: &File) -> Vec<File>;
    fn root(&self) -> File;
}