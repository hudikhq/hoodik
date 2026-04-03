//! Minimal tar archive extractor for parsing uncompressed POSIX/ustar archives.
//!
//! This parser extracts entries from a byte buffer — no crate dependencies needed.
//! It is the counterpart to `fs::tar` which builds the archives server-side.

use crate::error::{Error, Result};

/// A single entry extracted from a tar archive.
#[derive(Debug, Clone)]
pub struct TarEntry {
    /// Entry filename (e.g. `"000000.enc"`).
    pub name: String,
    /// Raw file data.
    pub data: Vec<u8>,
}

/// Extract all entries from an uncompressed tar archive.
///
/// Reads 512-byte headers, extracts name and size, reads the data, skips
/// padding. Stops at two consecutive zero blocks (end-of-archive) or end of
/// input.
pub fn extract_tar(archive: &[u8]) -> Result<Vec<TarEntry>> {
    let mut entries = Vec::new();
    let mut offset = 0;

    loop {
        // Need at least a 512-byte header.
        if offset + 512 > archive.len() {
            break;
        }

        let header = &archive[offset..offset + 512];

        // Two consecutive zero blocks signal end-of-archive.
        if header.iter().all(|&b| b == 0) {
            break;
        }

        // Parse filename from bytes 0..100 (null-terminated).
        let name_end = header[..100]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(100);
        let name = std::str::from_utf8(&header[..name_end])
            .map_err(|e| Error::Io(format!("invalid tar entry name: {e}")))?
            .to_string();

        // Parse size from bytes 124..135 (octal, null-terminated).
        let size_str = std::str::from_utf8(&header[124..135])
            .map_err(|e| Error::Io(format!("invalid tar size field: {e}")))?
            .trim_matches('\0')
            .trim();
        let size = u64::from_str_radix(size_str, 8)
            .map_err(|e| Error::Io(format!("invalid tar size value '{size_str}': {e}")))?;

        offset += 512; // Move past header.

        // Read file data.
        let data_end = offset + size as usize;
        if data_end > archive.len() {
            return Err(Error::Io(format!(
                "tar archive truncated: entry '{}' needs {} bytes at offset {}, but only {} remain",
                name,
                size,
                offset,
                archive.len() - offset
            )));
        }

        let data = archive[offset..data_end].to_vec();
        entries.push(TarEntry { name, data });

        // Skip past data + padding to next 512-byte boundary.
        let remainder = (size % 512) as usize;
        let padded_size = if remainder == 0 {
            size as usize
        } else {
            size as usize + (512 - remainder)
        };
        offset += padded_size;
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal tar archive from entries for testing.
    fn build_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut archive = Vec::new();
        for (name, data) in entries {
            // Build header using the same logic as fs::tar.
            let mut header = [0u8; 512];
            let name_bytes = name.as_bytes();
            let len = name_bytes.len().min(100);
            header[..len].copy_from_slice(&name_bytes[..len]);
            header[100..107].copy_from_slice(b"0000644");
            header[108..115].copy_from_slice(b"0000000");
            header[116..123].copy_from_slice(b"0000000");
            let size_octal = format!("{:011o}", data.len());
            header[124..135].copy_from_slice(size_octal.as_bytes());
            header[136..147].copy_from_slice(b"00000000000");
            header[156] = b'0';
            header[257..263].copy_from_slice(b"ustar\0");
            header[263..265].copy_from_slice(b"00");
            header[148..156].copy_from_slice(b"        ");
            let checksum: u32 = header.iter().map(|&b| b as u32).sum();
            let checksum_octal = format!("{:06o}\0 ", checksum);
            header[148..156].copy_from_slice(&checksum_octal.as_bytes()[..8]);

            archive.extend_from_slice(&header);
            archive.extend_from_slice(data);

            // Padding to 512-byte boundary.
            let remainder = data.len() % 512;
            if remainder != 0 {
                archive.extend(std::iter::repeat(0u8).take(512 - remainder));
            }
        }
        // End-of-archive: two zero blocks.
        archive.extend(std::iter::repeat(0u8).take(1024));
        archive
    }

    #[test]
    fn test_extract_empty_archive() {
        let archive = vec![0u8; 1024]; // Just end-of-archive marker.
        let entries = extract_tar(&archive).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_extract_single_entry() {
        let data = b"hello world";
        let archive = build_tar(&[("test.enc", data)]);
        let entries = extract_tar(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.enc");
        assert_eq!(entries[0].data, data);
    }

    #[test]
    fn test_extract_multiple_entries() {
        let data0 = vec![0xAA; 1024];
        let data1 = vec![0xBB; 2048];
        let data2 = vec![0xCC; 100];
        let archive = build_tar(&[
            ("000000.enc", &data0),
            ("000001.enc", &data1),
            ("000002.enc", &data2),
        ]);
        let entries = extract_tar(&archive).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "000000.enc");
        assert_eq!(entries[0].data, data0);
        assert_eq!(entries[1].name, "000001.enc");
        assert_eq!(entries[1].data, data1);
        assert_eq!(entries[2].name, "000002.enc");
        assert_eq!(entries[2].data, data2);
    }

    #[test]
    fn test_extract_exact_512_boundary_data() {
        let data = vec![0xFF; 512]; // Exactly 512 bytes — no padding needed.
        let archive = build_tar(&[("aligned.enc", &data)]);
        let entries = extract_tar(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].data.len(), 512);
    }

    #[test]
    fn test_extract_truncated_archive_returns_error() {
        let data = vec![0xAA; 1024];
        let mut archive = build_tar(&[("test.enc", &data)]);
        // Truncate in the middle of the data section.
        archive.truncate(512 + 500);
        let result = extract_tar(&archive);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_empty_file_entry() {
        let archive = build_tar(&[("empty.enc", b"")]);
        let entries = extract_tar(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "empty.enc");
        assert!(entries[0].data.is_empty());
    }

    #[test]
    fn test_roundtrip_large_data() {
        // Simulate a 4 MiB chunk.
        let data = vec![0x42; 4_194_304];
        let archive = build_tar(&[("000000.enc", &data)]);
        let entries = extract_tar(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].data.len(), 4_194_304);
        assert_eq!(entries[0].data, data);
    }
}
