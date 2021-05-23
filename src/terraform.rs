use std::{
    error::Error,
    fs::{set_permissions, File, Permissions},
    io::copy,
};

use regex::Regex;
use semver::Version;
use zip::ZipArchive;

use crate::{
    http,
    shared::{Binary, DotDir},
    utils,
};

const TF_RELEASES_URL: &str = "https://releases.hashicorp.com/terraform/";

pub fn list_available_versions() -> Result<String, Box<dyn Error>> {
    let http_client = http::client()?;
    let releases_html = http::get_text(&http_client, TF_RELEASES_URL, "text/html")?;
    let semver_regex = Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+")?;
    let mut versions: Vec<Version> = semver_regex
        .find_iter(&releases_html)
        .filter_map(|m| Version::parse(m.as_str()).ok())
        .collect();
    let result = utils::to_sorted_string(&mut versions);
    Ok(result)
}

pub fn install_binary_version(
    version: String,
    dot_dir: DotDir,
    os: String,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(Binary::TERRAFORM).join(&version);
    if !opt_file_path.exists() {
        let file_download_url = format!(
            "{0}{1}/terraform_{1}_{2}_amd64.zip",
            TF_RELEASES_URL, version, os
        );
        let shasums_download_url =
            format!("{0}{1}/terraform_{1}_SHA256SUMS", TF_RELEASES_URL, version);
        let http_client = http::client()?;
        let tmp_zip_file = tempfile::tempfile()?;
        http::get_bytes(&http_client, &file_download_url, &tmp_zip_file)?;
        let shasums = http::get_text(&http_client, &shasums_download_url, "text/plain")?;
        let sha256_regex = Regex::new(
            format!(r"([a-f0-9]+)\s+terraform_{}_{}_amd64.zip", &version, &os).as_str(),
        )?;
        let expected_sha256 = utils::get_capture_group(&sha256_regex, 1, &shasums)?;
        utils::check_sha256_sum(&tmp_zip_file, &expected_sha256)?;
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
