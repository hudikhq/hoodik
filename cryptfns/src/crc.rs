use crc::{Crc, CRC_16_IBM_SDLC};
pub const X25: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_SDLC);

/// Generate CRC16 digest
pub fn crc16_digest(input: &[u8]) -> String {
    format!("{:x}", X25.checksum(input))
}
