//! POSIX ustar header writer and streaming parser for chunk archives.
//!
//! The format is simple enough (512-byte fixed-width headers, one per entry,
//! two zero blocks at the end) that a hand-rolled implementation beats a crate
//! dependency — especially since every tar we emit or consume contains only
//! regular-file entries whose names and sizes we already control.

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

/// Outcome of one step of [`parse_next_entry`] against a growing input buffer.
#[derive(Debug)]
pub enum TarStep<'a> {
    /// One regular-file entry is available. `consumed` is the number of bytes
    /// the caller should drop from the front of their buffer before calling
    /// again. `data` borrows from the caller-supplied slice for this entry.
    Entry {
        name: String,
        data: &'a [u8],
        consumed: usize,
    },
    /// End-of-archive marker seen (two zero blocks). No more entries.
    End,
    /// Need more bytes before the next decision can be made.
    NeedMoreData,
    /// Malformed archive — invalid header, size that would overflow, etc.
    Malformed(String),
}

/// Parse at most one entry from the front of `buffer`.
///
/// Call repeatedly, dropping `consumed` bytes after each `Entry`, until
/// `End`. Unlike a one-shot parser this lets the caller feed a streaming
/// request body 512-byte-header-at-a-time without loading the full archive
/// into memory — only the current entry's payload lives in the buffer.
pub fn parse_next_entry(buffer: &[u8]) -> TarStep<'_> {
    if buffer.len() < 512 {
        return TarStep::NeedMoreData;
    }

    let header = &buffer[..512];

    if header.iter().all(|&b| b == 0) {
        return TarStep::End;
    }

    let name_end = header[..100].iter().position(|&b| b == 0).unwrap_or(100);
    let name = match std::str::from_utf8(&header[..name_end]) {
        Ok(s) => s.to_string(),
        Err(e) => return TarStep::Malformed(format!("invalid tar entry name utf-8: {e}")),
    };

    let size_bytes = &header[124..135];
    let size_str = match std::str::from_utf8(size_bytes) {
        Ok(s) => s.trim_matches('\0').trim(),
        Err(e) => return TarStep::Malformed(format!("invalid tar size field utf-8: {e}")),
    };
    let size = match u64::from_str_radix(size_str, 8) {
        Ok(n) => n,
        Err(e) => {
            return TarStep::Malformed(format!("invalid tar size value '{size_str}': {e}"));
        }
    };

    let padded = size as usize + tar_padding_len(size);
    let total = 512usize.saturating_add(padded);
    if buffer.len() < total {
        return TarStep::NeedMoreData;
    }

    let data = &buffer[512..512 + size as usize];
    TarStep::Entry {
        name,
        data,
        consumed: total,
    }
}

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

    /// Build a single-entry tar block suitable for parser tests.
    fn one_entry_tar(name: &str, data: &[u8]) -> Vec<u8> {
        let mut archive = Vec::new();
        archive.extend_from_slice(&tar_header(name, data.len() as u64));
        archive.extend_from_slice(data);
        archive.extend(std::iter::repeat_n(0u8, tar_padding_len(data.len() as u64)));
        archive
    }

    #[test]
    fn parse_entry_simple() {
        let mut archive = one_entry_tar("000000.enc", b"hello");
        archive.extend(std::iter::repeat_n(0u8, TAR_END_OF_ARCHIVE_LEN));

        match parse_next_entry(&archive) {
            TarStep::Entry {
                name,
                data,
                consumed,
            } => {
                assert_eq!(name, "000000.enc");
                assert_eq!(data, b"hello");
                // Header (512) + one padded data block (512) = 1024 consumed.
                assert_eq!(consumed, 1024);
                match parse_next_entry(&archive[consumed..]) {
                    TarStep::End => {}
                    other => panic!("expected End after sole entry, got {other:?}"),
                }
            }
            other => panic!("expected Entry, got {other:?}"),
        }
    }

    #[test]
    fn parse_entry_at_512_boundary() {
        // 512-byte payload has no padding; the consumed count must reflect that.
        let body = vec![0xABu8; 512];
        let archive = one_entry_tar("aligned.enc", &body);
        match parse_next_entry(&archive) {
            TarStep::Entry {
                data, consumed, ..
            } => {
                assert_eq!(data.len(), 512);
                assert_eq!(consumed, 1024);
            }
            other => panic!("expected Entry, got {other:?}"),
        }
    }

    #[test]
    fn parse_needs_more_data_when_header_short() {
        let archive = vec![0u8; 100];
        match parse_next_entry(&archive) {
            TarStep::NeedMoreData => {}
            other => panic!("expected NeedMoreData, got {other:?}"),
        }
    }

    #[test]
    fn parse_needs_more_data_when_body_short() {
        // Declare a 4 KiB entry but only feed the header.
        let mut header_only = tar_header("000000.enc", 4096).to_vec();
        // Pad past 512 so we know the branch hit is "body incomplete", not
        // "header incomplete".
        header_only.extend(std::iter::repeat_n(0u8, 100));
        match parse_next_entry(&header_only) {
            TarStep::NeedMoreData => {}
            other => panic!("expected NeedMoreData, got {other:?}"),
        }
    }

    #[test]
    fn parse_malformed_size_field() {
        let mut header = tar_header("000000.enc", 10);
        // Overwrite the size field with non-octal garbage to force parse failure.
        header[124..135].copy_from_slice(b"xxxxxxxxxxx");
        let mut archive = header.to_vec();
        archive.extend_from_slice(b"1234567890");
        archive.extend(std::iter::repeat_n(0u8, 512 - 10));
        match parse_next_entry(&archive) {
            TarStep::Malformed(_) => {}
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn parse_end_of_archive_block() {
        let archive = vec![0u8; 512];
        match parse_next_entry(&archive) {
            TarStep::End => {}
            other => panic!("expected End, got {other:?}"),
        }
    }

    #[test]
    fn parse_multiple_entries_in_sequence() {
        let mut archive = one_entry_tar("000000.enc", b"chunk-zero");
        archive.extend(one_entry_tar("000001.enc", b"chunk-one"));
        archive.extend(std::iter::repeat_n(0u8, TAR_END_OF_ARCHIVE_LEN));

        let mut cursor = 0usize;
        let mut seen = Vec::new();
        loop {
            match parse_next_entry(&archive[cursor..]) {
                TarStep::Entry {
                    name,
                    data,
                    consumed,
                } => {
                    seen.push((name, data.to_vec()));
                    cursor += consumed;
                }
                TarStep::End => break,
                other => panic!("unexpected step: {other:?}"),
            }
        }
        assert_eq!(
            seen,
            vec![
                ("000000.enc".to_string(), b"chunk-zero".to_vec()),
                ("000001.enc".to_string(), b"chunk-one".to_vec()),
            ]
        );
    }
}
