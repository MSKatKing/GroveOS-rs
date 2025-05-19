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
    let boot_info: u64;
    unsafe {
        asm!("mov {boot_info}, rdi", boot_info = out(reg) boot_info);
    }
    let boot_info = unsafe { &*(boot_info as *const UEFIBootInfo) };
    for c in 0..boot_info.framebuffer_size {
        unsafe {
            *boot_info.framebuffer.offset(c as isize) = 0xFFFFFFFF;
        }
    }
    
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { }
}