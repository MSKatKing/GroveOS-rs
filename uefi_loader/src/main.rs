#![no_main]
#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::arch::asm;
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
    
    // SAFETY: if the GraphicsOutput protocol opened successfully, then the framebuffer should contain a valid address
    let (framebuffer, width, height) = unsafe {
        let (width, height) = gop.current_mode_info().resolution();
        (core::slice::from_raw_parts_mut(gop.frame_buffer().as_mut_ptr() as *mut u32, height * gop.current_mode_info().stride()), width, height)
    };

    let pml4 = allocate_table();

    for phdr in &elf.program_headers {
        if phdr.p_type == PT_LOAD {
            // We need to copy the parts of the header to page-aligned spaces.
            // The kernel code doesn't really care where it's placed, since it'll just use the contiguous virtual space
            let pages = (phdr.p_memsz + 0x1000 - 1) / 0x1000;
            let allocated_space = boot::allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages as _).expect("Failed to allocate correct pages for kernel elf");
            
            for i in 0..((phdr.p_memsz + 0x1000 - 1) / 0x1000) {
                // SAFETY: both kernel_file and allocated_space are valid pointers, and the length is within bounds, so .offset and .copy_to_nonoverlapping are safe to use
                unsafe {
                    kernel_file.as_ptr().offset(i as isize * 0x1000 + phdr.p_offset as isize)
                        .copy_to_nonoverlapping(allocated_space.as_ptr().offset(i as isize * 0x1000), PAGE_SIZE);
                }

                map_page(pml4, phdr.p_vaddr + i * 0x1000, allocated_space.as_ptr() as u64 + i * 0x1000, PAGE_WRITE);
            }
        }
    }

    info!("Finished mapping kernel! Entry @ {:x}", elf.entry);
    
    map_static(pml4);
    
    info!("PML4 formatted for kernel.");
    
    for i in 0..(((framebuffer.len() * size_of::<u32>()) + 0x1000 - 1) / 0x1000) as u64 {
        map_page(pml4, framebuffer.as_ptr() as u64 + i * 0x1000, framebuffer.as_ptr() as u64 + i * 0x1000, PAGE_WRITE);
    }
    
    let boot_info = boot::allocate_pool(MemoryType::LOADER_DATA, size_of::<UEFIBootInfo>()).unwrap();
    let boot_info = boot_info.as_ptr() as *mut UEFIBootInfo;
    
    // SAFETY: allocate_pool returns a valid pointer, so dereferencing it is safe
    unsafe {
        (*boot_info).framebuffer = framebuffer.as_mut_ptr();
        (*boot_info).framebuffer_size = framebuffer.len();
        (*boot_info).framebuffer_width = width;
        (*boot_info).framebuffer_height = height;
    }

    let prev_map = boot::memory_map(MemoryType::LOADER_DATA).unwrap();

    let mut memsz = 0usize;

    let excluded_types = [
        MemoryType::RESERVED,
        MemoryType::UNUSABLE,
        MemoryType::PAL_CODE,
        MemoryType::PERSISTENT_MEMORY,
    ];

    for entry in prev_map.entries() {
        if excluded_types.contains(&entry.ty) { continue; } // Skip reserved memory
        memsz += entry.page_count as usize;
    }
    
    let memory_bitmap_size = memsz / 8;
    let memory_bitmap = boot::allocate_pool(MemoryType::LOADER_DATA, memory_bitmap_size).unwrap();
    let memory_bitmap = memory_bitmap.as_ptr();

    for entry in prev_map.entries() {
        if excluded_types.contains(&entry.ty) { continue; } // Skip reserved memory
        for i in 0..entry.page_count {
            map_page(pml4, entry.phys_start + i * PAGE_SIZE as u64, (if entry.virt_start == 0 { entry.phys_start } else { entry.virt_start }) + i * PAGE_SIZE as u64, PAGE_WRITE);
        }
    }
    
    unsafe {
        (*boot_info).memory_bitmap_size = memory_bitmap_size;
        (*boot_info).memory_bitmap = memory_bitmap;
    }

    info!("UEFI memory map copied!");
    info!("MemorySize found to be {}mb ({} bytes)", memsz * PAGE_SIZE / (1e+6 as usize), memsz * PAGE_SIZE);
    info!("BootInfo at {:x?}", boot_info);

    // SAFETY: the uefi crate should handle exiting boot services safely
    let _final_map = unsafe {
        boot::exit_boot_services(None)
    };
    
    // SAFETY: the asm! block is safe by only moving a value to a register.
    // SAFETY: elf.entry should contain the entrypoint to the kernel, so turning it into a fn pointer is okay
    let kernel_main: extern "C" fn() -> ! = unsafe {
        let pml4 = pml4 as *mut PageTable as u64;
        asm!("mov cr3, {pml4}", pml4 = in(reg) pml4);

        core::mem::transmute(elf.entry as *const ())
    };
    
    // SAFETY: this only moves the value into the register so is safe
    unsafe {
        asm!("mov rdi, {boot_info}", boot_info = in(reg) boot_info);
    }
    
    kernel_main();
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

fn allocate_table() -> &'static mut PageTable {
    let addr = boot::allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1).unwrap();
    let addr = addr.as_ptr() as *mut PageTable;
    
    // SAFETY: allocate_pages returns a valid pointer, so dereferencing it is okay
    let out = unsafe { &mut *addr };
    out.entries.fill(0);

    out
}

unsafe fn get_or_allocate_table(table: &mut PageTable, idx: usize, flags: u64) -> &'static mut PageTable {
    if table.entries[idx] & PAGE_PRESENT != 0 {
        let other = table.entries[idx] & !0xFFF;
        let other = other as *mut PageTable;
        
        // NOT SAFE: we can't guarantee that the pointer is valid, but we're required to assume that it is. Hence, why this function is labeled as unsafe
        &mut *other
    } else {
        let other = allocate_table();
        table.entries[idx] = other as *mut PageTable as u64 | flags;
        other
    }
}

fn map_page(pml4: &mut PageTable, virt: u64, phys: u64, flags: u64) {
    // SAFETY: thus far, the pml4 should have only been built by this function, so get_or_allocate_table gets the values it's expecting and is thus safe to use
    let pdpt = unsafe { get_or_allocate_table(pml4, page_table_index!(virt, 3), flags | PAGE_PRESENT) };
    let pd = unsafe { get_or_allocate_table(pdpt, page_table_index!(virt, 2), flags | PAGE_PRESENT) };
    let pt =  unsafe { get_or_allocate_table(pd, page_table_index!(virt, 1), flags | PAGE_PRESENT) };

    pt.entries[page_table_index!(virt, 0)] = (phys & !0xFFF) | PAGE_PRESENT | flags;
}

fn map_static(pml4: &mut PageTable) {
    const VIRT: u64 = 0xFFFF_FDFF_FFFF_E000;

    let pdpt = unsafe { get_or_allocate_table(pml4, page_table_index!(VIRT, 3), PAGE_WRITE | PAGE_PRESENT) };
    let pd = unsafe { get_or_allocate_table(pdpt, page_table_index!(VIRT, 2), PAGE_WRITE | PAGE_PRESENT) };
    let pt =  unsafe { get_or_allocate_table(pd, page_table_index!(VIRT, 1), PAGE_WRITE | PAGE_PRESENT) };
    
    let phys = pt as *const PageTable as u64;
    
    pt.entries[page_table_index!(VIRT, 0)] = (phys & !0xFFF) | PAGE_PRESENT | PAGE_WRITE;
}

#[repr(C)]
struct UEFIBootInfo {
    framebuffer: *mut u32,
    framebuffer_size: usize,
    framebuffer_width: usize,
    framebuffer_height: usize,

    pub memory_bitmap: *mut u8,
    pub memory_bitmap_size: usize,
}