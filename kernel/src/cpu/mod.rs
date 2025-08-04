use core::arch::asm;

pub mod idt;
pub mod gdt;

#[inline(always)]
pub fn outb(port: u16, data: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") data, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn inb(port: u16) -> u8 {
    let out;
    unsafe {
        asm!("in al, dx", out("al") out, in("dx") port, options(nomem, nostack));
    }
    out
}

#[inline(always)]
pub fn inw(port: u16) -> u16 {
    let out;
    unsafe {
        asm!("in ax, dx", out("ax") out, in("dx") port, options(nomem, nostack));
    }
    out
}

#[inline(always)]
pub fn io_wait() {
    for _ in 0..10 {
        outb(0x80, 0);
    }
}