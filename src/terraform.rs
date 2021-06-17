use std::{error::Error, fs::File, io::copy};

use pgp::{types::KeyTrait, Deserializable, SignedPublicKey, StandaloneSignature};
use regex::Regex;
use zip::ZipArchive;

use crate::{
    http::HttpClient,
    shared::{Binary, DotDir},
    utils,
};

use std::env::consts::EXE_SUFFIX;

pub fn install_binary_version(
    version: String,
    dot_dir: DotDir,
    os: String,
    arch: String,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(Binary::Terraform).join(&version);
    if !opt_file_path.exists() {
        let zip_download_url = format!(
            "{0}/{1}/terraform_{1}_{2}_{3}.zip",
            TF_RELEASES_DOWNLOAD_URL, version, os, arch
        );
        let tmp_zip_file = tempfile::tempfile()?;
        let http_client = HttpClient::new()?;
        http_client.download_file(&zip_download_url, &tmp_zip_file)?;
        verify_download_integrity(&version, &dot_dir, &os, &arch, &http_client, &tmp_zip_file)?;
        let mut zip_archive = ZipArchive::new(tmp_zip_file)?;
        let file_name = format!("terraform{}", EXE_SUFFIX);
        let mut binary_in_zip = zip_archive.by_name(&file_name)?;
        let mut opt_file = File::create(&opt_file_path)?;
        copy(&mut binary_in_zip, &mut opt_file)?;
        #[cfg(unix)]
        {
            use std::fs::{set_permissions, Permissions};
            use std::os::unix::fs::PermissionsExt;
            set_permissions(&opt_file_path, Permissions::from_mode(0o755))?;
        }
    }
    Ok(format!("Installed terraform {}", version))
}

fn verify_download_integrity(
    version: &str,
    dot_dir: &DotDir,
    os: &str,
    arch: &str,
    http_client: &HttpClient,
    zip_file: &File,
) -> Result<(), Box<dyn Error>> {
    let shasums_download_url = format!(
        "{0}/{1}/terraform_{1}_SHA256SUMS",
        TF_RELEASES_DOWNLOAD_URL, version
    );
    let shasums = http_client.get_text(&shasums_download_url)?;
    let pgp_public_key_path = dot_dir.etc.join("terraform.asc");
    if pgp_public_key_path.is_file() && pgp_public_key_path.metadata()?.permissions().readonly() {
        let pgp_public_key_file = File::open(pgp_public_key_path)?;
        let (public_key, _) = SignedPublicKey::from_armor_single(pgp_public_key_file)?;
        let pgp_key_id = &hex::encode(public_key.fingerprint()).to_uppercase()[32..];
        let shasums_sig_download_url = format!(
            "{0}/{1}/terraform_{1}_SHA256SUMS.{2}.sig",
            TF_RELEASES_DOWNLOAD_URL, version, &pgp_key_id
        );
        let signature_bytes = http_client.get_bytes(&shasums_sig_download_url)?;
        let signature = StandaloneSignature::from_bytes(&signature_bytes[..])?;
        utils::verify_detached_pgp_signature(&shasums.as_bytes(), &signature, &public_key)?;
    } else {
        eprint!("WARNING: Skipping PGP signature verification. See https://github.com/superblk/terve#setup{}", utils::NEWLINE);
    }
    let sha256_regex =
        Regex::new(format!(r"([a-f0-9]+)\s+terraform_{}_{}_{}.zip", version, os, arch).as_str())?;
    let expected_sha256 = utils::regex_capture_group(&sha256_regex, 1, &shasums)?;
    utils::check_sha256_sum(zip_file, &expected_sha256)?;
    Ok(())
}

pub const TF_GIT_REPOSITORY_URL: &str = "https://github.com/hashicorp/terraform";

const TF_RELEASES_DOWNLOAD_URL: &str = "https://releases.hashicorp.com/terraform";
