#![no_std]
#![no_main]

#![feature(alloc_error_handler)]
#![feature(ptr_as_ref_unchecked)]

extern crate alloc;

mod screen;
mod cpu;
mod mem;

use alloc::vec::Vec;
use crate::cpu::gdt::{install_gdt_defaults, lgdt};
use crate::cpu::idt::lidt;
use crate::screen::{framebuffer_writer, init_writer, FramebufferWriter};
use core::arch::asm;
use core::panic::PanicInfo;
use crate::mem::heap::metadata::HeapMetadata;
use crate::mem::page;

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
        
        (boot_info as *const UEFIBootInfo).read()
    };
    
    init_writer(FramebufferWriter::from(&boot_info));
    
    framebuffer_writer().clear();

    page::init_paging(&boot_info);

    // Point where all page functions can be used

    println!("Initializing GDT...");
    install_gdt_defaults();
    lgdt();
    
    println!("Initializing IDT...");
    lidt();
    
    unsafe { HeapMetadata::init_heap(); }

    // Point where all heap functions can be used.

    let mut test = Vec::<u8>::with_capacity(8);
    test.push(0);
    test.push(1);
    test.push(2);
    test.push(3);
    test.push(4);
    test.push(5);
    test.push(6);
    test.push(7);
    test.push(8);
    
    let mut a: Vec<u8> = Vec::with_capacity(1);
    a.push(24);
    a.push(15);
    println!("a: {:?}", a);
    
    test.resize(32, 0);
    
    println!("test: {:?}", test);

    println!("{:?}", unsafe { HeapMetadata::kernel() }[0]);
    
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