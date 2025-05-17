#![no_main]
#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use goblin::elf::Elf;
use goblin::elf::program_header::PT_LOAD;
use log::info;
use uefi::boot::{AllocateType, MemoryType, OpenProtocolAttributes, OpenProtocolParams, PAGE_SIZE};
use uefi::mem::memory_map::MemoryMap;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileHandle, FileInfo, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;
use x86_64::PhysAddr;
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::PhysFrame;

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

    let (elf, kernel_file) = {
        let header = boot::allocate_pool(MemoryType::LOADER_DATA, info.file_size() as _).unwrap();

        // SAFETY: the pointer that UEFI returns has the length passed in, so creating a fat pointer is ok
        let kernel_file = unsafe { core::slice::from_raw_parts_mut(header.as_ptr(), info.file_size() as _) };
        let header = unsafe { core::slice::from_raw_parts_mut(header.as_ptr(), info.file_size() as _) };

        // Trick here, since header and kernel_file point to the same spot, we only need to write to one
        // Having both just gets around the compiler complaining about header being borrowed by Elf::parse()
        let _ = kernel.read(header);

        (Elf::parse(header).expect("Kernel corrupt"), kernel_file)
    };
    
    info!("Getting Graphics info...");

    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    
    // SAFETY: we're just getting information, not changing anything (like the mode) so shared access is fine
    let mut gop = unsafe {
        boot::open_protocol::<GraphicsOutput>(OpenProtocolParams { handle: gop_handle, agent: boot::image_handle(), controller: None }, OpenProtocolAttributes::GetProtocol)
    }.expect("Failed to get GraphicsOutput");
    
    info!("Opened Graphics Output");
    
    let framebuffer = unsafe {
        let (_, height) = gop.current_mode_info().resolution();
        core::slice::from_raw_parts_mut(gop.frame_buffer().as_mut_ptr() as *mut u32, height * gop.current_mode_info().stride())
    };

    let pml4 = unsafe { allocate_table() };

    let prev_map = boot::memory_map(MemoryType::LOADER_DATA).unwrap();

    let mut memsz = 0usize;

    let excluded_types = [
        MemoryType::RESERVED,
        MemoryType::MMIO,
        MemoryType::MMIO_PORT_SPACE,
        MemoryType::UNUSABLE,
        MemoryType::PAL_CODE,
        MemoryType::PERSISTENT_MEMORY,
    ];

    for entry in prev_map.entries() {
        if excluded_types.contains(&entry.ty) { continue; } // Skip reserved memory
        memsz += entry.page_count as usize;
        for i in 0..entry.page_count {
            unsafe {
                map_page(pml4, entry.phys_start + i * PAGE_SIZE as u64, (if entry.virt_start == 0 { entry.phys_start } else { entry.virt_start }) + i * PAGE_SIZE as u64, PAGE_WRITE);
            }
        }
    }

    info!("UEFI memory map copied!");
    info!("MemorySize found to be {}mb ({} bytes)", memsz * PAGE_SIZE / (1e+6 as usize), memsz * PAGE_SIZE);

    for phdr in &elf.program_headers {
        if phdr.p_type == PT_LOAD {
            for i in 0..((phdr.p_memsz + 0x1000 - 1) / 0x1000) {
                unsafe {
                    map_page(pml4, phdr.p_vaddr + i * 0x1000, (kernel_file.as_ptr() as u64) + phdr.p_offset + i * 0x1000, PAGE_WRITE);
                }
            }
        }
    }

    info!("Finished mapping kernel! Entry @ {:x}", elf.entry);

    let _final_map = unsafe {
        boot::exit_boot_services(None)
    };
    
    let boot_info = UEFIBootInfo {
        framebuffer,
    };
    
    unsafe {
        Cr3::write(PhysFrame::containing_address(PhysAddr::new(pml4 as *mut PageTable as _)), Cr3Flags::empty());

        core::mem::transmute::<u64, unsafe extern "C" fn(UEFIBootInfo)>(elf.entry)(boot_info);
    }

    loop { }
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

struct UEFIBootInfo {
    framebuffer: &'static mut [u32],
}