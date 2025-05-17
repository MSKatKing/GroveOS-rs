#![no_std]
#![no_main]

use core::panic::PanicInfo;

unsafe extern "C" {
    static __kernel_vstart: *const u64;
    static __kernel_vend: *const u64;
}

struct UEFIBootInfo {
    framebuffer: &'static mut [u32],
}

#[unsafe(no_mangle)]
pub extern "C" fn main(boot_info: UEFIBootInfo) -> ! {
    for c in boot_info.framebuffer {
        *c = 0;
    }
    
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { }
}