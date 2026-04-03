//! Minimal POSIX ustar tar header builder for streaming chunk archives.
//!
//! We hand-build headers instead of pulling in a crate dependency because the
//! format is simple (512-byte fixed-width struct) and we only need to write
//! regular file entries with known sizes.

/// Build a POSIX ustar tar header for a regular file entry.
///
/// The returned 512-byte block can be written directly before the file data.
/// After the data, write [`tar_padding`] zero bytes to reach a 512-byte boundary.
pub fn tar_header(filename: &str, size: u64) -> [u8; 512] {
    let mut header = [0u8; 512];

    // name (0..100)
    let name_bytes = filename.as_bytes();
    let len = name_bytes.len().min(100);
    header[..len].copy_from_slice(&name_bytes[..len]);

    // mode (100..108) — 0000644\0
    header[100..107].copy_from_slice(b"0000644");

    // uid (108..116) — 0000000\0
    header[108..115].copy_from_slice(b"0000000");

    // gid (116..124) — 0000000\0
    header[116..123].copy_from_slice(b"0000000");

    // size (124..136) — 11-digit octal + \0
    let size_octal = format!("{:011o}", size);
    header[124..135].copy_from_slice(size_octal.as_bytes());

    // mtime (136..148) — 0
    header[136..147].copy_from_slice(b"00000000000");

    // typeflag (156) — '0' = regular file
    header[156] = b'0';

    // magic (257..263) — "ustar\0"
    header[257..263].copy_from_slice(b"ustar\0");

    // version (263..265) — "00"
    header[263..265].copy_from_slice(b"00");

    // Compute checksum: sum of all bytes, treating the checksum field (148..156)
    // as ASCII spaces (0x20).
    // First, fill checksum field with spaces for the calculation.
    header[148..156].copy_from_slice(b"        ");

    let checksum: u32 = header.iter().map(|&b| b as u32).sum();
    let checksum_octal = format!("{:06o}\0 ", checksum);
    header[148..156].copy_from_slice(&checksum_octal.as_bytes()[..8]);

    header
}

/// Return the number of zero-padding bytes needed after `size` bytes of data
/// to reach the next 512-byte boundary.
pub fn tar_padding_len(size: u64) -> usize {
    let remainder = (size % 512) as usize;
    if remainder == 0 {
        0
    } else {
        512 - remainder
    }
}

/// Two 512-byte zero blocks that signal end-of-archive.
pub const TAR_END_OF_ARCHIVE_LEN: usize = 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tar_header_checksum_valid() {
        let header = tar_header("test.enc", 1024);

        // Verify checksum: parse stored checksum, recompute with spaces, compare.
        let stored = std::str::from_utf8(&header[148..154])
            .unwrap()
            .trim();
        let stored_checksum = u32::from_str_radix(stored, 8).unwrap();

        let mut check_header = header;
        check_header[148..156].copy_from_slice(b"        ");
        let computed: u32 = check_header.iter().map(|&b| b as u32).sum();

        assert_eq!(stored_checksum, computed);
    }

    #[test]
    fn test_tar_header_filename() {
        let header = tar_header("000001.enc", 4096);
        let name = std::str::from_utf8(&header[..10]).unwrap();
        assert_eq!(name, "000001.enc");
        // Rest of name field should be zeros
        assert!(header[10..100].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_tar_header_size_encoding() {
        let header = tar_header("test.enc", 4_194_304); // 4 MiB
        let size_str = std::str::from_utf8(&header[124..135]).unwrap();
        let parsed_size = u64::from_str_radix(size_str, 8).unwrap();
        assert_eq!(parsed_size, 4_194_304);
    }

    #[test]
    fn test_tar_header_ustar_magic() {
        let header = tar_header("test.enc", 0);
        assert_eq!(&header[257..263], b"ustar\0");
        assert_eq!(&header[263..265], b"00");
    }

    #[test]
    fn test_tar_header_typeflag_regular_file() {
        let header = tar_header("test.enc", 0);
        assert_eq!(header[156], b'0');
    }

    #[test]
    fn test_tar_padding_len_exact_multiple() {
        assert_eq!(tar_padding_len(0), 0);
        assert_eq!(tar_padding_len(512), 0);
        assert_eq!(tar_padding_len(1024), 0);
    }

    #[test]
    fn test_tar_padding_len_non_multiple() {
        assert_eq!(tar_padding_len(1), 511);
        assert_eq!(tar_padding_len(511), 1);
        assert_eq!(tar_padding_len(513), 511);
        assert_eq!(tar_padding_len(100), 412);
    }

    #[test]
    fn test_tar_padding_len_4mib() {
        // 4 MiB is an exact multiple of 512
        assert_eq!(tar_padding_len(4_194_304), 0);
    }

    #[test]
    fn test_tar_end_of_archive_len() {
        assert_eq!(TAR_END_OF_ARCHIVE_LEN, 1024);
    }
}
