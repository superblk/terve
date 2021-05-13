use argh::FromArgs;
use home::home_dir;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::{error::Error, io::Cursor, process};
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
use semver::Version;

// TODO: implement SHA256 validation
// TODO: support macos
// TODO: tests
// TODO: implement GPG verification (terraform)
// TODO: support windows

static TF_RELEASES_URL: &str = "https://releases.hashicorp.com/terraform/";

static TG_RELEASES_API_URL: &str = "https://api.github.com/repos/gruntwork-io/terragrunt/releases";

static TG_RELEASES_DOWNLOAD_URL: &str =
    "https://github.com/gruntwork-io/terragrunt/releases/download/";

static HTTP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

#[tokio::main]
async fn main() {
    process::exit(match run().await {
        Ok(msg) => {
            println!("{}", msg);
            0
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    });
}

async fn run() -> Result<String, Box<dyn Error>> {
    let args: Args = argh::from_env();
    match home_dir() {
        Some(h) => {
            let dot_dir_path = create_dot_dir(h)?;
            match (args.action, args.binary, args.version_spec) {
                (Action::LIST, binary, Some(v)) if v == "local" => {
                    list_installed_versions(binary, dot_dir_path)
                }
                (Action::LIST, Binary::TERRAFORM, version_opt) => {
                    list_available_terraform_versions(as_prefix(version_opt)).await
                }
                (Action::LIST, Binary::TERRAGRUNT, version_opt) => {
                    list_available_terragrunt_versions(as_prefix(version_opt)).await
                }
                (Action::INSTALL, Binary::TERRAFORM, Some(version)) => {
                    install_terraform_version(version, dot_dir_path).await
                }
                (Action::INSTALL, Binary::TERRAGRUNT, Some(version)) => {
                    install_terragrunt_version(version, dot_dir_path).await
                }
                (Action::SELECT, binary, Some(version)) => {
                    select_binary_version(binary, version, dot_dir_path)
                }
                (Action::REMOVE, binary, Some(version)) => {
                    remove_binary_version(binary, version, dot_dir_path)
                }
                _ => Err("Invalid arguments. Run 'terve --help' for usage".into()),
            }
        }
        None => Err("Unable to resolve user home directory".into()),
    }
}

fn as_prefix(version_opt: Option<String>) -> String {
    version_opt.unwrap_or("".to_string())
}

fn create_dot_dir(home_dir: PathBuf) -> Result<String, Box<dyn Error>> {
    let dot_dir_path = format!("{}/.terve", home_dir.display());
    create_dir_all(format!("{}/bin", &dot_dir_path))?;
    create_dir_all(format!("{}/opt/terraform", &dot_dir_path))?;
    create_dir_all(format!("{}/opt/terragrunt", &dot_dir_path))?;
    Ok(dot_dir_path)
}

fn new_http_client() -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder().user_agent(HTTP_USER_AGENT).build()?)
}

fn list_installed_versions(binary: Binary, dot_dir_path: String) -> Result<String, Box<dyn Error>> {
    let opt_dir_path = format!("{}/opt/{}", dot_dir_path, binary);
    let mut installed_versions: Vec<Version> = read_dir(&opt_dir_path)?
        .filter_map(|r| Some(r.ok()?.path().strip_prefix(&opt_dir_path).ok()?.to_path_buf()))
        .filter_map(|p| Version::parse(p.display().to_string().as_str()).ok())
        .collect();
    installed_versions.sort();
    installed_versions.reverse();
    let result = match installed_versions.len() {
        0 => "No matching terraform versions found".to_string(),
        _ => installed_versions.into_iter().map(|v| v.to_string()).collect::<Vec<String>>().join("\n"),
    };
    Ok(result)
}

async fn list_available_terraform_versions(
    version_prefix: String,
) -> Result<String, Box<dyn Error>> {
    let http_client = new_http_client()?;
    let http_response = http_client
        .get(TF_RELEASES_URL)
        .header("Accept", "text/html")
        .send()
        .await?
        .error_for_status()?;
    let releases_html = http_response.text().await?;
    let semver_regex = Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+").unwrap();
    let mut versions: Vec<&str> = semver_regex
        .find_iter(&releases_html)
        .map(|mat| mat.as_str())
        .filter(|v| v.starts_with(version_prefix.as_str()))
        .collect();
    versions.dedup();
    let result = match versions.len() {
        0 => "No matching terraform versions found".to_string(),
        _ => versions.join("\n"),
    };
    Ok(result)
}

async fn list_available_terragrunt_versions(
    version_prefix: String,
) -> Result<String, Box<dyn Error>> {
    let http_client = new_http_client()?;
    let mut releases: Vec<GitHubRelease> = Vec::new();
    // Max out at 1000 most recent releases
    for page_num in 1..=10 {
        let http_response = http_client
            .get(TG_RELEASES_API_URL)
            .header("Accept", "application/vnd.github.v3+json")
            .query(&[("per_page", "100")])
            .query(&[("page", page_num.to_string().as_str())])
            .send()
            .await?
            .error_for_status()?;
        let mut page = http_response.json::<Vec<GitHubRelease>>().await?;
        let num_results = page.len();
        releases.append(&mut page);
        if num_results < 100 {
            break;
        }
    }
    let versions: Vec<&str> = releases
        .iter()
        .map(|r| r.tag_name.trim_start_matches('v'))
        .filter(|v| v.starts_with(version_prefix.as_str()))
        .collect();
    let result = match versions.len() {
        0 => "No matching terragrunt versions found".to_string(),
        _ => versions.join("\n"),
    };
    Ok(result)
}

async fn install_terraform_version(
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let download_url = format!(
        "{0}{1}/terraform_{1}_linux_amd64.zip",
        TF_RELEASES_URL, version
    );
    let http_client = new_http_client()?;
    let http_response = http_client
        .get(download_url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?
        .error_for_status()?;
    let zip_file_bytes = http_response.bytes().await?;
    let mut cursor = Cursor::new(zip_file_bytes);
    let mut tmp_zip_file = tempfile::tempfile()?;
    copy(&mut cursor, &mut tmp_zip_file)?;
    let mut zip_archive = ZipArchive::new(tmp_zip_file)?;
    let mut binary_in_zip = zip_archive.by_name("terraform")?;
    let bin_path = format!("{}/opt/terraform/{}", dot_dir_path, version);
    let mut bin_file = File::create(&bin_path)?;
    copy(&mut binary_in_zip, &mut bin_file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        set_permissions(&bin_path, Permissions::from_mode(0o755))?;
    }
    Ok(format!("Installed terraform {}", version))
}

async fn install_terragrunt_version(
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let download_url = format!(
        "{0}/v{1}/terragrunt_linux_amd64",
        TG_RELEASES_DOWNLOAD_URL, version
    );
    let http_client = new_http_client()?;
    let http_response = http_client
        .get(download_url)
        .header("Accept", "application/octet-stream")
        .send()
        .await?
        .error_for_status()?;
    let bin_file_bytes = http_response.bytes().await?;
    let mut cursor = Cursor::new(bin_file_bytes);
    let bin_path = format!("{}/opt/terragrunt/{}", dot_dir_path, version);
    let mut bin_file = File::create(&bin_path)?;
    copy(&mut cursor, &mut bin_file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        set_permissions(&bin_path, Permissions::from_mode(0o755))?;
    }
    Ok(format!("Installed terragrunt {}", version))
}

fn select_binary_version(
    binary: Binary,
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let symlink_path = format!("{}/bin/{}", dot_dir_path, binary);
    let bin_path = format!("{}/opt/{}/{}", dot_dir_path, binary, version);
    if !Path::new(&bin_path).exists() {
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
        std::os::unix::fs::symlink(&bin_path, &symlink_path)?;
    }
    Ok(format!("Using {} {}", binary, version))
}

fn remove_binary_version(
    binary: Binary,
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let symlink_path = format!("{}/bin/{}", dot_dir_path, binary);
    let bin_path = format!("{}/opt/{}/{}", dot_dir_path, binary, version);
    if Path::new(&bin_path).exists() {
        remove_file(&bin_path)?;
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
                "binary must be one of: tf (alias: terraform) or tg (alias: terragrunt)"
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
