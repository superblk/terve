use std::{
    error::Error,
    fs::File,
    io::{copy, Seek, SeekFrom},
};

use crate::{
    http::HttpClient,
    shared::{Binary, DotDir},
    utils::{check_sha256_sum, regex_capture_group, wprintln},
};
use regex::Regex;
use reqwest::StatusCode;
use std::env::consts::EXE_SUFFIX;

pub fn install_binary_version(
    version: String,
    dot_dir: DotDir,
    os: String,
    arch: String,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(Binary::Terragrunt).join(&version);
    if !opt_file_path.exists() {
        let file_name = format!("terragrunt_{}_{}{}", os, arch, EXE_SUFFIX);
        let file_download_url = format!("{}/v{}/{}", TG_RELEASES_DOWNLOAD_URL, version, file_name);
        let shasums_download_url =
            format!("{0}/v{1}/SHA256SUMS", TG_RELEASES_DOWNLOAD_URL, version);
        let mut tmp_file = tempfile::tempfile()?;
        let http_client = HttpClient::new()?;
        http_client.download_file(&file_download_url, &tmp_file)?;
        match http_client.get_text(&shasums_download_url) {
            Ok(shasums) => {
                let sha256_regex = Regex::new(format!(r"([a-f0-9]+)\s+{}", file_name).as_str())?;
                let expected_sha256 = regex_capture_group(&sha256_regex, 1, &shasums)?;
                check_sha256_sum(&tmp_file, &expected_sha256)?;
            }
            Err(e) if e.status() == Some(StatusCode::NOT_FOUND) => {
                wprintln("Skipping SHA256 file integrity check. See https://github.com/superblk/terve#install");
            }
            Err(other) => {
                return Err(other.into());
            }
        }
        tmp_file.seek(SeekFrom::Start(0))?;
        let mut opt_file = File::create(&opt_file_path)?;
        copy(&mut tmp_file, &mut opt_file)?;
        #[cfg(unix)]
        {
            use std::fs::{set_permissions, Permissions};
            use std::os::unix::fs::PermissionsExt;
            set_permissions(&opt_file_path, Permissions::from_mode(0o755))?;
        }
    }
    Ok(format!("Installed terragrunt {}", version))
}

pub const TG_GIT_REPOSITORY_URL: &str = "https://github.com/gruntwork-io/terragrunt";

const TG_RELEASES_DOWNLOAD_URL: &str =
    "https://github.com/gruntwork-io/terragrunt/releases/download";
