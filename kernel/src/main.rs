#![no_std]
#![no_main]

mod screen;
mod cpu;

use core::arch::{asm, naked_asm};
use core::panic::PanicInfo;
use crate::cpu::gdt::{install_gdt_defaults, lgdt};
use crate::cpu::idt::lidt;
use crate::screen::{framebuffer_writer, init_writer, FramebufferWriter};

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
    
    println!("Initializing GDT...");
    install_gdt_defaults();
    lgdt();
    
    println!("Initializing IDT...");
    lidt();
    
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    
    loop { }
}