use crate::io::ata::{ATA_PIO_DEVICE_PRIMARY, ATA_PIO_DEVICE_SECONDARY};

mod ata;

const DEVICES: &[&'static dyn DiskDevice] = &[&ATA_PIO_DEVICE_PRIMARY, &ATA_PIO_DEVICE_SECONDARY];

pub trait DiskDevice {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]) -> Result<(), DiskError>;
    fn write_sector(&self, lba: u64, buffer: &[u8]) -> Result<(), DiskError>;
}

pub enum DiskError {
    NotSupported,
}