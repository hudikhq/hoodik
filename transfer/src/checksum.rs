/// Compute CRC16 checksum (IBM SDLC / X.25) and return it as a hex string.
pub fn crc16(data: &[u8]) -> String {
    cryptfns::crc::crc16_digest(data)
}
