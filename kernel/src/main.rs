#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

unsafe extern "C" {
    static __kernel_vstart: *const u64;
    static __kernel_vend: *const u64;
}

#[repr(C)]
pub struct UEFIBootInfo {
    framebuffer: *mut u32,
    framebuffer_size: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // SAFETY: rdi contains the address for the UEFIBootInfo passed in from the bootloader, so dereferencing the pointer is ok
    let boot_info = unsafe {
        let boot_info: u64;
        asm!("mov {boot_info}, rdi", boot_info = out(reg) boot_info);
        
        & *(boot_info as *const UEFIBootInfo)
    };
    
    let framebuffer = unsafe {
        core::slice::from_raw_parts_mut(boot_info.framebuffer, boot_info.framebuffer_size)
    };
    
    for c in framebuffer {
        *c = 0xFFFFFFFF;
    }
    
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { }
}