#![no_main]
#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use goblin::elf::Elf;
use log::info;
use uefi::boot::MemoryType;
use uefi::boot::{AllocateType, MemoryType, PAGE_SIZE};
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

        Elf::parse(header).expect("Kernel corrupt")
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

#[repr(align(0x1000))]
struct PageTable {
    entries: [u64; 512]
}

macro_rules! page_table_index {
    ($addr:expr, $depth:expr) => {
        ((($addr >> (12 + 9 *  $depth)) & 0x1FF) as usize)
    };
}

const PAGE_PRESENT: u64 = 1 << 0;
const PAGE_WRITE: u64 = 1 << 1;

unsafe fn allocate_table() -> &'static mut PageTable {
    let addr = boot::allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1).unwrap();
    let addr = addr.as_ptr() as *mut PageTable;
    
    let out = &mut *addr;
    for i in 0..512 {
        out.entries[i] = 0;
    }

    out
}

unsafe fn get_or_allocate_table(table: &mut PageTable, idx: usize, flags: u64) -> &'static mut PageTable {
    if table.entries[idx] & PAGE_PRESENT != 0 {
        let other = table.entries[idx] & !0xFFF;
        let other = other as *mut PageTable;
        &mut *other
    } else {
        let other = allocate_table();
        table.entries[idx] = other as *mut PageTable as u64 | flags;
        other
    }
}

unsafe fn map_page(pml4: &mut PageTable, virt: u64, phys: u64, flags: u64) {
    let pdpt = get_or_allocate_table(pml4, page_table_index!(virt, 3), flags | PAGE_PRESENT);
    let pd = get_or_allocate_table(pdpt, page_table_index!(virt, 2), flags | PAGE_PRESENT);
    let pt =  get_or_allocate_table(pd, page_table_index!(virt, 1), flags | PAGE_PRESENT);

    pt.entries[page_table_index!(virt, 0)] = (phys & !0xFFF) | PAGE_PRESENT | flags;
}