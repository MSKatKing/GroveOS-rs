pub trait DiskDevice {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]) -> Result<(), DiskError>;
    fn write_sector(&self, lba: u64, buffer: &[u8]) -> Result<(), DiskError>;
}

pub enum DiskError {

}