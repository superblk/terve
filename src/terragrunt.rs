use std::{error::Error, fs::File, io::copy};

use crate::{
    http::HttpClient,
    shared::{Binary, DotDir},
    utils,
};
use regex::Regex;
use reqwest::StatusCode;
use semver::{Prerelease, Version};

pub fn list_available_versions() -> Result<String, Box<dyn Error>> {
    let mut versions: Vec<Version> = utils::git_list_remote_tags(TG_GIT_REPOSITORY_URL)?
        .iter()
        .map(|t| t.trim_start_matches('v'))
        .filter_map(|s| Version::parse(s).ok())
        .filter(|v| v.pre == Prerelease::EMPTY)
        .collect();
    let result = utils::to_sorted_multiline_string(&mut versions);
    Ok(result)
}

pub fn install_binary_version(
    version: String,
    dot_dir: DotDir,
    os: String,
    arch: String,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(Binary::Terragrunt).join(&version);
    if !opt_file_path.exists() {
        let file_download_url = format!(
            "{}/v{}/terragrunt_{}_{}",
            TG_RELEASES_DOWNLOAD_URL, version, os, arch
        );
        let shasums_download_url =
            format!("{0}/v{1}/SHA256SUMS", TG_RELEASES_DOWNLOAD_URL, version);
        let http_client = HttpClient::new()?;
        let mut tmp_file = tempfile::tempfile()?;
        http_client.download_file(&file_download_url, &tmp_file)?;
        match http_client.get_text(&shasums_download_url, "text/plain") {
            Ok(shasums) => {
                let sha256_regex =
                    Regex::new(format!(r"([a-f0-9]+)\s+terragrunt_{}_{}", os, arch).as_str())?;
                let expected_sha256 = utils::regex_capture_group(&sha256_regex, 1, &shasums)?;
                utils::check_sha256_sum(&tmp_file, &expected_sha256)?;
            }
            Err(e) if e.status() == Some(StatusCode::NOT_FOUND) => {
                eprintln!("WARNING: Skipping SHA256 file integrity check. See https://github.com/superblk/terve#install");
            }
            Err(other) => {
                return Err(other.into());
            }
        }
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

const TG_GIT_REPOSITORY_URL: &str = "https://github.com/gruntwork-io/terragrunt";

const TG_RELEASES_DOWNLOAD_URL: &str =
    "https://github.com/gruntwork-io/terragrunt/releases/download";
