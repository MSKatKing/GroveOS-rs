#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(ptr_as_ref_unchecked)]
#![feature(abi_x86_interrupt)]
extern crate alloc;

mod cpu;
mod mem;
mod screen;

use alloc::vec::Vec;
// use alloc::vec::Vec;
use crate::cpu::gdt::{install_gdt_defaults, lgdt};
use crate::cpu::idt::{lidt, setup_idt};
use crate::mem::heap::metadata::HeapMetadata;
use crate::mem::page;
use crate::screen::{FramebufferWriter, framebuffer_writer, init_writer};
use core::arch::asm;
use core::panic::PanicInfo;
use crate::cpu::acpi::Rsdp;

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
    
    pub acpi_rsdp: *const u8,
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

    println!("Initializing GDT...");
    install_gdt_defaults();
    lgdt();

    println!("Initializing IDT...");
    setup_idt();
    lidt();

    page::init_paging(&boot_info);

    // Point where all page functions can be used

    unsafe {
        HeapMetadata::init_heap();
    }

    framebuffer_writer().clear();

    // Point where all heap functions can be used.

    cpu::acpi::register_default_systems();

    unsafe {
        cpu::acpi::parse_rsdp(boot_info.acpi_rsdp as *const Rsdp);
    }

    cpu::print_cpu_info();

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    loop {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    println!("allocation error: {:?}", layout);

    loop {}
}
