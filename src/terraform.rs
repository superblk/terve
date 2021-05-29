use std::{
    error::Error,
    fs::{set_permissions, File, Permissions},
    io::copy,
};

use pgp::{types::KeyTrait, Deserializable, SignedPublicKey, StandaloneSignature};
use regex::Regex;
use semver::Version;
use zip::ZipArchive;

use crate::{
    http::HttpClient,
    shared::{Binary, DotDir},
    utils,
};

pub fn list_available_versions() -> Result<String, Box<dyn Error>> {
    let http_client = HttpClient::new()?;
    let releases_html = http_client.get_text(TF_RELEASES_URL, "text/html")?;
    let semver_regex = Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+")?;
    let mut versions: Vec<Version> = semver_regex
        .find_iter(&releases_html)
        .filter_map(|m| Version::parse(m.as_str()).ok())
        .collect();
    let result = utils::to_sorted_multiline_string(&mut versions);
    Ok(result)
}

pub fn install_binary_version(
    version: String,
    dot_dir: DotDir,
    os: String,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(Binary::Terraform).join(&version);
    if !opt_file_path.exists() {
        let zip_download_url = format!(
            "{0}{1}/terraform_{1}_{2}_amd64.zip",
            TF_RELEASES_URL, version, os
        );
        let http_client = HttpClient::new()?;
        let tmp_zip_file = tempfile::tempfile()?;
        http_client.download_file(&zip_download_url, &tmp_zip_file)?;
        verify_download_integrity(&version, &dot_dir, &os, &http_client, &tmp_zip_file)?;
        let mut zip_archive = ZipArchive::new(tmp_zip_file)?;
        let mut binary_in_zip = zip_archive.by_name("terraform")?;
        let mut opt_file = File::create(&opt_file_path)?;
        copy(&mut binary_in_zip, &mut opt_file)?;
        #[cfg(unix)]
        {
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
    http_client: &HttpClient,
    zip_file: &File,
) -> Result<(), Box<dyn Error>> {
    let shasums_download_url = format!("{0}{1}/terraform_{1}_SHA256SUMS", TF_RELEASES_URL, version);
    let shasums = http_client.get_text(&shasums_download_url, "text/plain")?;
    let pgp_public_key_path = dot_dir.etc.join("terraform.asc");
    if pgp_public_key_path.is_file() && pgp_public_key_path.metadata()?.permissions().readonly() {
        let pgp_public_key_file = File::open(pgp_public_key_path)?;
        let (public_key, _) = SignedPublicKey::from_armor_single(pgp_public_key_file)?;
        let pgp_key_fingerprint = hex::encode(public_key.fingerprint()).to_uppercase();
        let shasums_sig_download_url = format!(
            "{0}{1}/terraform_{1}_SHA256SUMS.{2}.sig",
            TF_RELEASES_URL,
            version,
            &pgp_key_fingerprint[32..]
        );
        let signature_bytes = http_client.get_bytes(&shasums_sig_download_url)?;
        let signature = StandaloneSignature::from_bytes(&signature_bytes[..])?;
        utils::verify_detached_pgp_signature(&shasums, &signature, &public_key)?;
    } else {
        eprintln!(
            "WARN: Skipping PGP signature verification (please install {})",
            pgp_public_key_path.display()
        );
    }
    let sha256_regex =
        Regex::new(format!(r"([a-f0-9]+)\s+terraform_{}_{}_amd64.zip", version, os).as_str())?;
    let expected_sha256 = utils::regex_capture_group(&sha256_regex, 1, &shasums)?;
    utils::check_sha256_sum(zip_file, &expected_sha256)?;
    Ok(())
}

const TF_RELEASES_URL: &str = "https://releases.hashicorp.com/terraform/";
