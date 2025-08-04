use crate::cpu;

const ATA_PRIMARY_IO: u16 = 0x1F0;
const ATA_REG_DATA: u16 = 0x00;
const ATA_REG_ERROR: u16 = 0x01;
const ATA_REG_SECCOUNT0: u16 = 0x02;
const ATA_REG_LBA0: u16 = 0x03;
const ATA_REG_LBA1: u16 = 0x04;
const ATA_REG_LBA2: u16 = 0x05;
const ATA_REG_HDDEVSEL: u16 = 0x06;
const ATA_REG_COMMAND: u16 = 0x07;
const ATA_REG_STATUS: u16 = 0x07;
const ATA_CMD_READ_PIO: u8 = 0x20;
const ATA_SR_BSY: u8 = 0x80;
const ATA_SR_DRQ: u8 = 0x08;

fn read_status() -> u8 {
    cpu::inb(ATA_PRIMARY_IO + ATA_REG_STATUS)
}

fn ata_wait_bsy() {
    while read_status() & ATA_SR_BSY != 0 { }
}

fn ata_wait_drq() {
    while read_status() & ATA_SR_DRQ != 0 { }
}

pub fn ata_read_sector(lba: u32, buffer: &mut [u8]) {
    ata_wait_bsy();
    
    cpu::outb(ATA_PRIMARY_IO + ATA_REG_SECCOUNT0, 1);
    cpu::outb(ATA_PRIMARY_IO + ATA_REG_LBA0, lba as u8);
    cpu::outb(ATA_PRIMARY_IO + ATA_REG_LBA1, (lba >> 8) as u8);
    cpu::outb(ATA_PRIMARY_IO + ATA_REG_LBA2, (lba >> 16) as u8);
    cpu::outb(ATA_PRIMARY_IO + ATA_REG_HDDEVSEL, 0xE0 | ((lba >> 24) & 0x0F) as u8);
    cpu::outb(ATA_PRIMARY_IO + ATA_REG_COMMAND, ATA_CMD_READ_PIO);
    
    ata_wait_bsy();
    ata_wait_drq();
    
    let buffer = buffer.as_mut_ptr().cast::<u16>();
    for i in 0..256 {
        unsafe {
            core::ptr::write(buffer.add(i), cpu::inw(ATA_PRIMARY_IO + ATA_REG_DATA));
        }
    }
    
    cpu::io_wait();
}

pub fn ata_read_sectors(lba: u32, buffer: &mut [u8]) {
    for (idx, chunk) in buffer.chunks_exact_mut(512).enumerate() {
        ata_read_sector(lba + idx as u32, chunk);
    }
}