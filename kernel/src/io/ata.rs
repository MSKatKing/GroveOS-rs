use crate::io::{DiskDevice, DiskError};

pub(super) const ATA_PIO_DEVICE_PRIMARY: AtaPioDevice = AtaPioDevice {
    io_base: 0x1F0,
    control_base: 0x3F6,
    master: true,
};

pub(super) const ATA_PIO_DEVICE_SECONDARY: AtaPioDevice = AtaPioDevice {
    io_base: 0x170,
    control_base: 0x376,
    master: false,
};

pub struct AtaPioDevice {
    pub io_base: u16,
    pub control_base: u16,
    pub master: bool,
}

impl DiskDevice for AtaPioDevice {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]) -> Result<(), DiskError> {
        todo!()
    }

    fn write_sector(&self, lba: u64, buffer: &[u8]) -> Result<(), DiskError> {
        todo!()
    }
}