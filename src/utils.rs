use std::{
    error::Error,
    fs::File,
    io::{copy, Seek, SeekFrom},
};

use regex::Regex;
use sha2::{Digest, Sha256};

pub fn check_sha256_sum(mut file: &File, expected: &str) -> Result<(), Box<dyn Error>> {
    file.seek(SeekFrom::Start(0))?;
    let mut sha256 = Sha256::new();
    copy(&mut file, &mut sha256)?;
    let result = sha256.finalize();
    let actual = hex::encode(result);
    if &actual != expected {
        Err(format!(
            "File sha256 checksum mismatch: expected '{}', got '{}'",
            expected, actual
        ))?;
    }
    Ok(())
}

pub fn get_capture_group(
    regex: &Regex,
    group: usize,
    text: &str,
) -> Result<String, Box<dyn Error>> {
    let result = regex
        .captures(text)
        .ok_or("Regex capture group failed")?
        .get(group)
        .ok_or("Regex capture group not found")?
        .as_str()
        .to_string();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_match() {
        let file = File::open("tests/special.txt").expect("failed to open test file");
        check_sha256_sum(
            &file,
            "b93e557fb1f4b32346b3e035985c25017356d99cce0b98140fbbd225fe57f185",
        )
        .expect("expected sha256 to match");
    }

    #[test]
    fn test_sha256_mismatch() {
        let file = File::open("tests/special.txt").expect("failed to open test file");
        check_sha256_sum(
            &file,
            "a93e557fb1f4b32346b3e035985c25017356d99cce0b98140fbbd225fe57f185",
        )
        .expect_err("expected sha256 to mismatch");
    }

    #[test]
    fn test_regex_capture() {
        let str_match = "abc123 hai";
        let str_no_match = "nope";
        let regex = Regex::new(r"([a-z0-9]+) hai").unwrap();
        assert_eq!(get_capture_group(&regex, 1, &str_match).unwrap(), "abc123");
        assert!(get_capture_group(&regex, 1, &str_no_match).is_err());
    }
}
