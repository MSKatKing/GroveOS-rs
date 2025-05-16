#![no_main]
#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use goblin::elf::Elf;
use log::info;
use uefi::boot::MemoryType;
use uefi::prelude::*;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileHandle, FileInfo, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        boot::allocate_pool(MemoryType::LOADER_DATA, layout.size()).unwrap().as_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        boot::free_pool(NonNull::new(ptr).unwrap()).unwrap();
    }
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let mut kernel = load_kernel().expect("Failed to load the kernel");

    let mut file_info = [0u8;0];
    let err = kernel.get_info::<FileInfo>(&mut file_info).err().unwrap();
    let file_info = boot::allocate_pool(MemoryType::LOADER_DATA, err.data().unwrap()).unwrap();

    // SAFETY: the pointer that UEFI returns has the length passed in, so creating a fat pointer is ok
    let file_info = unsafe { core::slice::from_raw_parts_mut(file_info.as_ptr(), err.data().unwrap()) };
    let info = kernel.get_info::<FileInfo>(file_info).unwrap();

    let mut kernel = kernel.into_regular_file().unwrap();

    let _elf = {
        let header = boot::allocate_pool(MemoryType::LOADER_DATA, info.file_size() as _).unwrap();

        // SAFETY: the pointer that UEFI returns has the length passed in, so creating a fat pointer is ok
        let header = unsafe { core::slice::from_raw_parts_mut(header.as_ptr(), info.file_size() as _) };
        let _ = kernel.read(header);

        Elf::parse(header)
    };

    info!("Hello, world! (kernel loaded)");
    boot::stall(10_000_000);
    Status::SUCCESS
}

fn load_kernel() -> Option<FileHandle> {
    let image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).ok()?;
    let mut fs = boot::open_protocol_exclusive::<SimpleFileSystem>(image.device()?).ok()?;

    let mut directory = fs.open_volume().ok()?;

    directory.open(cstr16!("kernel.elf"), FileMode::Read, FileAttribute::empty()).ok()
}