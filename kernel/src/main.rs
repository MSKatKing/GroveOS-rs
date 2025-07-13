#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

mod screen;
mod cpu;
mod mem;

use alloc::vec::Vec;
// use alloc::vec::Vec;
use crate::cpu::gdt::{install_gdt_defaults, lgdt, GDTEntry};
use crate::cpu::idt::lidt;
use crate::mem::page_allocator::FrameAllocator;
use crate::mem::paging::PageTable;
use crate::screen::{framebuffer_writer, init_writer, FramebufferWriter};
use core::arch::asm;
use core::panic::PanicInfo;

unsafe extern "C" {
    static __kernel_vstart: *const u64;
    static __kernel_vend: *const u64;
}

#[repr(C)]
pub struct UEFIBootInfo {
    pub framebuffer: *mut u32,
    pub framebuffer_size: usize,
    pub framebuffer_width: usize,
    pub framebuffer_height: usize,
    
    pub memory_bitmap: *mut u8,
    pub memory_bitmap_size: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // SAFETY: this is okay since were only disabling interrupts
    unsafe {
        asm!("cli");
    }
    
    // SAFETY: rdi contains the address for the UEFIBootInfo passed in from the bootloader, so dereferencing the pointer is ok
    let boot_info = unsafe {
        let boot_info: u64;
        asm!("mov {boot_info}, rdi", boot_info = out(reg) boot_info);
        
        & *(boot_info as *const UEFIBootInfo)
    };
    
    init_writer(FramebufferWriter::from(boot_info));
    
    framebuffer_writer().clear();
    
    FrameAllocator::init(boot_info);

    println!("Initializing GDT...");
    install_gdt_defaults();
    lgdt();
    
    println!("Initializing IDT...");
    lidt();
    
    // Point where all page functions can be used
    
    let pml4 = PageTable::current();
    
    let new_pml4 = PageTable::new();

    for i in (0u64..0x10000000).step_by(0x1000) {
        new_pml4.get_mut(i)
            .map_to(i)
            .set_writable(true);
    }

    for i in (0..boot_info.framebuffer_size as u64).step_by(0x1000) {
        new_pml4.get_mut(i + boot_info.framebuffer as u64)
            .map_to(i + boot_info.framebuffer as u64)
            .set_writable(true);
    }
    
    new_pml4[511] = pml4[511];
    
    new_pml4.install();

    println!("Initialized PML4...");

    // Point where all heap functions can be used.

    let mut test = Vec::<u8>::with_capacity(3);
    test.push(0);
    
    println!("{:?}", test);
    
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    
    loop { }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    println!("allocation error: {:?}", layout);

    loop { }
}