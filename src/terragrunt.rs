use std::{
    error::Error,
    fs::{set_permissions, File, Permissions},
};

use crate::{
    http,
    shared::{Binary, DotDir},
    utils,
};
use regex::Regex;
use semver::Version;
use serde::Deserialize;

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

const TG_RELEASES_API_URL: &str = "https://api.github.com/repos/gruntwork-io/terragrunt/releases";

const TG_RELEASES_DOWNLOAD_URL: &str =
    "https://github.com/gruntwork-io/terragrunt/releases/download/";

pub fn list_available_versions() -> Result<String, Box<dyn Error>> {
    let http_client = http::client()?;
    let mut releases: Vec<GitHubRelease> = Vec::new();
    // Max out at 500 most recent releases
    for page_num in 1..=5 {
        let mut page: Vec<GitHubRelease> = http_client
            .get(TG_RELEASES_API_URL)
            .header("Accept", "application/vnd.github.v3+json")
            .query(&[("per_page", "100")])
            .query(&[("page", page_num.to_string().as_str())])
            .send()?
            .error_for_status()?
            .json()?;
        let num_results = page.len();
        releases.append(&mut page);
        if num_results < 100 {
            break;
        }
    }
    let mut versions: Vec<Version> = releases
        .iter()
        .map(|r| r.tag_name.trim_start_matches("v"))
        .filter_map(|s| Version::parse(s).ok())
        .filter(|v| !v.is_prerelease())
        .collect();
    let result = utils::to_sorted_string(&mut versions);
    Ok(result)
}

pub fn install_binary_version(
    version: String,
    dot_dir: DotDir,
    os: String,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(Binary::TERRAGRUNT).join(&version);
    if !opt_file_path.exists() {
        let file_download_url = format!(
            "{}/v{}/terragrunt_{}_amd64",
            TG_RELEASES_DOWNLOAD_URL, version, os
        );
        let shasums_download_url =
            format!("{0}/v{1}/SHA256SUMS", TG_RELEASES_DOWNLOAD_URL, version);
        let http_client = http::client()?;
        let opt_file = File::create(&opt_file_path)?;
        http::get_bytes(&http_client, &file_download_url, &opt_file)?;
        let shasums = http::get_text(&http_client, &shasums_download_url, "text/plain")?;
        let sha256_regex = Regex::new(format!(r"([a-f0-9]+)\s+terragrunt_{}_amd64", os).as_str())?;
        let expected_sha256 = utils::get_capture_group(&sha256_regex, 1, &shasums)?;
        utils::check_sha256_sum(&File::open(&opt_file_path)?, &expected_sha256)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            set_permissions(&opt_file_path, Permissions::from_mode(0o755))?;
        }
    }
    Ok(format!("Installed terragrunt {}", version))
}
