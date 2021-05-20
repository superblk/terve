use argh::FromArgs;
use home::home_dir;
use regex::Regex;
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env::consts::OS;
use std::{
    error::Error,
    io::{Seek, SeekFrom},
    process,
    time::Duration,
};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use std::{
    fs::{create_dir_all, read_dir, read_link, remove_file, File},
    path::Path,
};
use std::{
    fs::{set_permissions, Permissions},
    io::copy,
};
use zip::ZipArchive;

fn main() {
    process::exit(match run() {
        Ok(s) => {
            if !s.is_empty() {
                println!("{}", s);
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    });
}

type StringOrError = Result<String, Box<dyn Error>>;

fn run() -> StringOrError {
    let args: Args = argh::from_env();
    let os = match OS {
        "linux" => "linux",
        "macos" => "darwin",
        os => panic!("Unsupported OS: {}", os),
    };
    match home_dir() {
        Some(home) => {
            let dot_dir = DotDir::init(home)?;
            match (args.action, args.binary, args.version_spec) {
                (Action::LIST, binary, None) => list_installed_versions(binary, dot_dir),
                (Action::LIST, Binary::TERRAFORM, Some(v)) if v == "r" || v == "remote" => {
                    list_available_terraform_versions()
                }
                (Action::LIST, Binary::TERRAGRUNT, Some(v)) if v == "r" || v == "remote" => {
                    list_available_terragrunt_versions()
                }
                (Action::INSTALL, Binary::TERRAFORM, Some(version))
                    if Version::parse(&version).is_ok() =>
                {
                    install_terraform_version(version, dot_dir, os)
                }
                (Action::INSTALL, Binary::TERRAGRUNT, Some(version))
                    if Version::parse(&version).is_ok() =>
                {
                    install_terragrunt_version(version, dot_dir, os)
                }
                (Action::SELECT, binary, Some(version)) if Version::parse(&version).is_ok() => {
                    select_binary_version(binary, version, dot_dir)
                }
                (Action::REMOVE, binary, Some(version)) if Version::parse(&version).is_ok() => {
                    remove_binary_version(binary, version, dot_dir)
                }
                _ => Err("Invalid arguments. Run 'terve --help' for usage".into()),
            }
        }
        None => Err("Unable to resolve user home directory".into()),
    }
}

struct DotDir {
    bin: PathBuf,
    opt: PathBuf,
}

impl DotDir {
    fn init(home_dir: PathBuf) -> Result<DotDir, Box<dyn Error>> {
        let bin_dir = home_dir.join(".terve/bin");
        let opt_dir = home_dir.join(".terve/opt");
        create_dir_all(&bin_dir)?;
        create_dir_all(opt_dir.join("terraform"))?;
        create_dir_all(opt_dir.join("terragrunt"))?;
        Ok(DotDir {
            bin: bin_dir,
            opt: opt_dir,
        })
    }
}

fn list_installed_versions(binary: Binary, dot_dir: DotDir) -> StringOrError {
    let opt_dir = dot_dir.opt.join(binary);
    let mut installed_versions: Vec<Version> = read_dir(&opt_dir)?
        .filter_map(|r| Some(r.ok()?.path().strip_prefix(&opt_dir).ok()?.to_path_buf()))
        .filter_map(|p| Version::parse(p.display().to_string().as_str()).ok())
        .collect();
    installed_versions.sort();
    installed_versions.reverse();
    let result = installed_versions
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join("\n");
    Ok(result)
}

fn list_available_terraform_versions() -> StringOrError {
    let http_client = new_http_client()?;
    let releases_html = http_get_text(&http_client, TF_RELEASES_URL, "text/html")?;
    let semver_regex = Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+").unwrap();
    let mut versions: Vec<&str> = semver_regex
        .find_iter(&releases_html)
        .map(|mat| mat.as_str())
        .collect();
    versions.dedup();
    let result = versions.join("\n");
    Ok(result)
}

fn list_available_terragrunt_versions() -> StringOrError {
    let http_client = new_http_client()?;
    let mut releases: Vec<GitHubRelease> = Vec::new();
    // Max out at 1000 most recent releases
    for page_num in 1..=10 {
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
    let versions: Vec<&str> = releases
        .iter()
        .map(|r| r.tag_name.trim_start_matches("v"))
        .collect();
    let result = versions.join("\n");
    Ok(result)
}

fn install_terraform_version(version: String, dot_dir: DotDir, os: &str) -> StringOrError {
    let file_download_url = format!(
        "{0}{1}/terraform_{1}_{2}_amd64.zip",
        TF_RELEASES_URL, version, os
    );
    let shasums_download_url = format!("{0}{1}/terraform_{1}_SHA256SUMS", TF_RELEASES_URL, version);
    let http_client = new_http_client()?;
    let tmp_zip_file = tempfile::tempfile()?;
    http_get_bytes(&http_client, &file_download_url, &tmp_zip_file)?;
    let shasums = http_get_text(&http_client, &shasums_download_url, "text/plain")?;
    let sha256_regex =
        Regex::new(format!(r"([a-f0-9]+)\s+terraform_{}_{}_amd64.zip", &version, &os).as_str())
            .unwrap();
    let expected_sha256 = capture_group(&sha256_regex, 1, &shasums)?;
    check_sha256(&tmp_zip_file, &expected_sha256)?;
    let mut zip_archive = ZipArchive::new(tmp_zip_file)?;
    let mut binary_in_zip = zip_archive.by_name("terraform")?;
    let opt_file_path = dot_dir.opt.join(Binary::TERRAFORM).join(&version);
    let mut opt_file = File::create(&opt_file_path)?;
    copy(&mut binary_in_zip, &mut opt_file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        set_permissions(&opt_file_path, Permissions::from_mode(0o755))?;
    }
    Ok(format!("Installed terraform {}", version))
}

fn install_terragrunt_version(version: String, dot_dir: DotDir, os: &str) -> StringOrError {
    let file_download_url = format!(
        "{}/v{}/terragrunt_{}_amd64",
        TG_RELEASES_DOWNLOAD_URL, version, os
    );
    let shasums_download_url = format!("{0}/v{1}/SHA256SUMS", TG_RELEASES_DOWNLOAD_URL, version);
    let http_client = new_http_client()?;
    let opt_file_path = dot_dir.opt.join(Binary::TERRAGRUNT).join(&version);
    let opt_file = File::create(&opt_file_path)?;
    http_get_bytes(&http_client, &file_download_url, &opt_file)?;
    let shasums = http_get_text(&http_client, &shasums_download_url, "text/plain")?;
    let sha256_regex =
        Regex::new(format!(r"([a-f0-9]+)\s+terragrunt_{}_amd64", os).as_str()).unwrap();
    let expected_sha256 = capture_group(&sha256_regex, 1, &shasums)?;
    check_sha256(&File::open(&opt_file_path)?, &expected_sha256)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        set_permissions(&opt_file_path, Permissions::from_mode(0o755))?;
    }
    Ok(format!("Installed terragrunt {}", version))
}

fn select_binary_version(binary: Binary, version: String, dot_dir: DotDir) -> StringOrError {
    let symlink_path = dot_dir.bin.join(&binary);
    let opt_file_path = dot_dir.opt.join(&binary).join(&version);
    if !Path::new(&opt_file_path).exists() {
        Err(format!(
            "{0} version {1} is not installed. Run 'terve install {0} {1}'",
            binary, version
        ))?
    }
    if read_link(&symlink_path).is_ok() {
        remove_file(&symlink_path)?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&opt_file_path, &symlink_path)?;
    }
    Ok(format!("Using {} {}", binary, version))
}

fn remove_binary_version(binary: Binary, version: String, dot_dir: DotDir) -> StringOrError {
    let symlink_path = dot_dir.bin.join(&binary);
    let opt_file_path = dot_dir.opt.join(&binary).join(&version);
    if Path::new(&opt_file_path).exists() {
        remove_file(&opt_file_path)?;
        let symlink_is_broken =
            read_link(&symlink_path).is_ok() && !Path::new(&symlink_path).exists();
        if symlink_is_broken {
            remove_file(&symlink_path)?;
        }
    }
    Ok(format!("Removed {} {}", binary, version))
}

#[derive(FromArgs)]
/// Unified terraform and terragrunt version manager
struct Args {
    #[argh(positional)]
    action: Action,

    #[argh(positional)]
    binary: Binary,

    #[argh(positional)]
    version_spec: Option<String>,
}

enum Action {
    LIST,
    INSTALL,
    SELECT,
    REMOVE,
}

enum Binary {
    TERRAFORM,
    TERRAGRUNT,
}

impl FromStr for Action {
    type Err = String;

    fn from_str(a: &str) -> Result<Self, Self::Err> {
        match a {
            "l" | "list" => Ok(Action::LIST),
            "i" | "install" => Ok(Action::INSTALL),
            "s" | "select" => Ok(Action::SELECT),
            "r" | "remove" => Ok(Action::REMOVE),
            _ => Err(format!(
                "action must be one of: l[ist], i[nstall], s[elect] or r[remove]"
            )),
        }
    }
}

impl FromStr for Binary {
    type Err = String;

    fn from_str(a: &str) -> Result<Self, Self::Err> {
        match a {
            "tf" | "terraform" => Ok(Binary::TERRAFORM),
            "tg" | "terragrunt" => Ok(Binary::TERRAGRUNT),
            _ => Err(format!(
                "binary must be one of: tf, terraform, tg, terragrunt"
            )),
        }
    }
}

impl Display for Binary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Binary::TERRAFORM => write!(f, "terraform"),
            Binary::TERRAGRUNT => write!(f, "terragrunt"),
        }
    }
}

impl AsRef<Path> for Binary {
    fn as_ref(&self) -> &Path {
        let path = match *self {
            Binary::TERRAFORM => "terraform",
            Binary::TERRAGRUNT => "terragrunt",
        };
        Path::new(path)
    }
}

static TF_RELEASES_URL: &str = "https://releases.hashicorp.com/terraform/";

static TG_RELEASES_API_URL: &str = "https://api.github.com/repos/gruntwork-io/terragrunt/releases";

static TG_RELEASES_DOWNLOAD_URL: &str =
    "https://github.com/gruntwork-io/terragrunt/releases/download/";

static HTTP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

fn new_http_client() -> Result<Client, Box<dyn Error>> {
    let client = Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .https_only(true)
        .build()?;
    Ok(client)
}

fn http_get_bytes(
    http_client: &Client,
    url: &str,
    mut dest_file: &File,
) -> Result<u64, Box<dyn Error>> {
    let num_bytes = http_client
        .get(url)
        .header("Accept", "application/octet-stream")
        .send()?
        .copy_to(&mut dest_file)?;
    Ok(num_bytes)
}

fn http_get_text(http_client: &Client, url: &str, accept: &str) -> StringOrError {
    let text = http_client
        .get(url)
        .header("Accept", accept)
        .send()?
        .error_for_status()?
        .text()?;
    Ok(text)
}

fn check_sha256(mut file: &File, expected: &str) -> Result<(), Box<dyn Error>> {
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

fn capture_group(regex: &Regex, group: usize, text: &str) -> StringOrError {
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
    fn test_good_sha256() {
        let file = File::open("tests/special.txt").expect("failed to open test file");
        check_sha256(&file, "b93e557fb1f4b32346b3e035985c25017356d99cce0b98140fbbd225fe57f185").expect("expected sha256 match");
    }

    #[test]
    fn test_bad_sha256() {
        let file = File::open("tests/special.txt").expect("failed to open test file");
        check_sha256(&file, "a93e557fb1f4b32346b3e035985c25017356d99cce0b98140fbbd225fe57f185").expect_err("expected sha256 mismatch");
    }
}
