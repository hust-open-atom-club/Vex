use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub fn sha256_hex_of_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn sha256_hex_of_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn sha256_hex_of_bytes_matches_known_abc_vector() {
        assert_eq!(
            sha256_hex_of_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_hex_of_bytes_empty_matches_known_vector() {
        assert_eq!(
            sha256_hex_of_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hex_of_file_matches_bytes_for_same_payload() {
        let payload: Vec<u8> = (0..20_000u32).map(|i| (i % 251) as u8).collect();
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&payload).unwrap();
        tmp.flush().unwrap();
        let from_file = sha256_hex_of_file(tmp.path()).unwrap();
        let from_bytes = sha256_hex_of_bytes(&payload);
        assert_eq!(from_file, from_bytes);
        assert_eq!(from_file.len(), 64);
    }
}
