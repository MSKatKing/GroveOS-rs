use alloc::string::String;
use alloc::vec::Vec;

mod ata;
mod fat32;

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
    pub fn open(path: &str) -> Option<Self> {
        todo!()
    }
    
    pub fn read(&mut self) -> &[u8] {
        todo!()
    }
    
    pub fn list_children(&self) -> Vec<File> {
        todo!()
    }
    
    pub fn is_directory(&self) -> bool {
        self.kind == FileKind::Directory
    }
    
    pub fn filename(&self) -> &str {
        let trimmed = self.path.trim_end_matches('/');
        trimmed.rsplit('/').next().unwrap_or("")
    }
}

trait FileSystem {
    fn open(&self, path: &str) -> Option<File>;
    fn read_file<'a>(&self, file: &'a mut File) -> Option<&'a [u8]>;
    fn list_dir(&self, dir: &File) -> Vec<File>;
    fn root(&self) -> File;
}