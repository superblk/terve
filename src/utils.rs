use git2::{Direction, Remote};
use pgp::{types::KeyTrait, SignedPublicKey, StandaloneSignature};
use regex::Regex;
use semver::Version;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::File,
    io::{copy, Seek, SeekFrom},
};

pub fn check_sha256_sum(mut file: &File, expected_sha256: &str) -> Result<(), Box<dyn Error>> {
    file.seek(SeekFrom::Start(0))?;
    let mut sha256 = Sha256::new();
    copy(&mut file, &mut sha256)?;
    let result = sha256.finalize();
    let actual_sha256 = hex::encode(result);
    if actual_sha256 != expected_sha256 {
        return Err(format!(
            "File sha256 checksum mismatch: expected '{}', got '{}'",
            expected_sha256, actual_sha256
        )
        .into());
    }
    Ok(())
}

pub fn regex_capture_group(
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

pub fn to_sorted_multiline_string(versions: &mut Vec<Version>) -> String {
    versions.sort();
    versions.dedup();
    versions.reverse();
    let result = versions
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join(NEWLINE);
    result
}

pub fn verify_detached_pgp_signature(
    content: &[u8],
    signature: &StandaloneSignature,
    public_key: &SignedPublicKey,
) -> Result<(), Box<dyn Error>> {
    if public_key.is_signing_key() && signature.verify(&public_key, content).is_ok() {
        return Ok(());
    } else {
        for sub_key in &public_key.public_subkeys {
            if sub_key.is_signing_key() && signature.verify(sub_key, content).is_ok() {
                return Ok(());
            }
        }
    }
    Err("PGP signature verification failed".into())
}

pub fn git_list_remote_tags(repo_url: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut remote = Remote::create_detached(repo_url)?;
    remote.connect(Direction::Fetch)?;
    let result = remote
        .list()?
        .iter()
        .map(|h| h.name().to_string())
        .filter(|r| r.starts_with("refs/tags/") && !r.ends_with("^{}"))
        .map(|s| s.trim_start_matches("refs/tags/").to_owned())
        .collect();
    remote.disconnect()?;
    Ok(result)
}

#[cfg(unix)]
pub const NEWLINE: &str = "\n";

#[cfg(windows)]
pub const NEWLINE: &str = "\r\n";

#[cfg(test)]
mod tests {

    use std::fs::read_to_string;

    use pgp::Deserializable;

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
        assert_eq!(
            regex_capture_group(&regex, 1, &str_match).unwrap(),
            "abc123"
        );
        assert!(regex_capture_group(&regex, 1, &str_no_match).is_err());
    }

    #[test]
    fn test_version_sort() {
        let mut versions = vec!["0.13.4", "0.15.4", "0.1.0", "0.13.4"]
            .into_iter()
            .filter_map(|s| Version::parse(s).ok())
            .collect();
        assert_eq!(
            format!("0.15.4{0}0.13.4{0}0.1.0", NEWLINE),
            to_sorted_multiline_string(&mut versions)
        );
    }

    #[test]
    fn test_pgp_verify_match() {
        let content = read_to_string("tests/terraform_0.13.1_SHA256SUMS").unwrap();
        let public_key =
            SignedPublicKey::from_armor_single(File::open("tests/hashicorp-72D7468F.asc").unwrap())
                .unwrap()
                .0;
        let signature = StandaloneSignature::from_bytes(
            File::open("tests/terraform_0.13.1_SHA256SUMS.72D7468F.sig").unwrap(),
        )
        .unwrap();
        assert!(verify_detached_pgp_signature(content.as_bytes(), &signature, &public_key).is_ok());
    }

    #[test]
    fn test_pgp_verify_mismatch() {
        let content = read_to_string("tests/terraform_0.13.1_SHA256SUMS").unwrap();
        let public_key =
            SignedPublicKey::from_armor_single(File::open("tests/hashicorp-72D7468F.asc").unwrap())
                .unwrap()
                .0;
        let signature = StandaloneSignature::from_bytes(
            File::open("tests/terraform_0.13.1_SHA256SUMS.348FFC4C.sig").unwrap(),
        )
        .unwrap();
        assert!(
            verify_detached_pgp_signature(content.as_bytes(), &signature, &public_key).is_err()
        );
    }

    #[test]
    fn test_git_list_remote_tags() {
        let tags = git_list_remote_tags("https://github.com/gruntwork-io/terragrunt").unwrap();
        assert!(tags.contains(&"v0.29.7".to_string()));
    }
}
