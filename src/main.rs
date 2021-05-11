use argh::FromArgs;
use home::home_dir;
use regex::Regex;
use reqwest::Client;
use std::{error::Error, io::Cursor, process};
use std::{
    fs::{create_dir_all, read_link, remove_file, File},
    path::Path,
};
use std::{
    fs::{set_permissions, Permissions},
    io::copy,
};
use std::{path::PathBuf, str::FromStr};
use zip::ZipArchive;
use serde::Deserialize;

// TODO: terragrunt
// TODO: implement SHA256 validation
// TODO: support macos
// TODO: tests
// TODO: show which versions are installed (in list)
// TODO: implement GPG verification (terraform)

static TF_RELEASES_URL: &str = "https://releases.hashicorp.com/terraform/";

static TG_RELEASES_URL: &str = "https://api.github.com/repos/gruntwork-io/terragrunt/releases";

static HTTP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Deserialize)]
struct Release {
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
            match (args.action, args.binary, args.version) {
                (Action::LIST, Binary::TERRAFORM, version_opt) => {
                    do_list_terraform_versions(version_opt.unwrap_or("".to_string())).await
                }
                (Action::LIST, Binary::TERRAGRUNT, version_opt) => {
                    do_list_terragrunt_versions(version_opt.unwrap_or("".to_string())).await
                }
                (Action::INSTALL, Binary::TERRAFORM, Some(version)) => {
                    do_install_terraform_version(version, dot_dir_path).await
                }
                (Action::SELECT, Binary::TERRAFORM, Some(version)) => {
                    do_select_terraform_version(version, dot_dir_path)
                }
                (Action::REMOVE, Binary::TERRAFORM, Some(version)) => {
                    do_remove_terraform_version(version, dot_dir_path)
                }
                _ => Err("Invalid arguments. Run 'terve --help' for usage".into()),
            }
        }
        None => Err("Unable to resolve user home directory".into()),
    }
}

fn create_dot_dir(home_dir: PathBuf) -> Result<String, Box<dyn Error>> {
    let dot_dir_path = format!("{}/.terve", home_dir.display());
    create_dir_all(format!("{}/bin", &dot_dir_path))?;
    create_dir_all(format!("{}/opt", &dot_dir_path))?;
    Ok(dot_dir_path)
}

fn new_reqwest_client() -> Result<Client, Box<dyn Error>> {
    let client = Client::builder().user_agent(HTTP_USER_AGENT).build()?;
    Ok(client)
}

async fn do_list_terraform_versions(version_prefix: String) -> Result<String, Box<dyn Error>> {
    let http_client = new_reqwest_client()?;
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

async fn do_list_terragrunt_versions(
    version_prefix: String,
) -> Result<String, Box<dyn Error>> {
    let http_client = new_reqwest_client()?;
    let http_response = http_client
        .get(TG_RELEASES_URL)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?
        .error_for_status()?;
    let releases = http_response.json::<Vec<Release>>().await?;
    let versions: Vec<&str> = releases.iter().map(|r| r.tag_name.trim_start_matches('v')).filter(|v| v.starts_with(version_prefix.as_str())).collect();
    let result = match versions.len() {
        0 => "No matching terragrunt versions found".to_string(),
        _ => versions.join("\n") ,
    };
    Ok(result)
}

async fn do_install_terraform_version(
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let download_url = format!(
        "{0}{1}/terraform_{1}_linux_amd64.zip",
        TF_RELEASES_URL, version
    );
    let http_client = new_reqwest_client()?;
    let http_response = http_client
        .get(download_url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?
        .error_for_status()?;
    let zip_file_bytes = http_response.bytes().await?;
    let mut cursor = Cursor::new(zip_file_bytes);
    let mut temp_zip_file = tempfile::tempfile()?;
    copy(&mut cursor, &mut temp_zip_file)?;
    let mut zip_archive = ZipArchive::new(temp_zip_file)?;
    let mut binary_in_zip = zip_archive.by_name("terraform")?;
    let bin_path = format!("{}/opt/terraform_{}", dot_dir_path, version);
    let mut bin_file = File::create(&bin_path)?;
    copy(&mut binary_in_zip, &mut bin_file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = binary_in_zip.unix_mode() {
            set_permissions(&bin_path, Permissions::from_mode(mode))?;
        }
    }
    Ok(format!("Installed terraform {}", version))
}

fn do_select_terraform_version(
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let bin_path = format!("{}/opt/terraform_{}", dot_dir_path, version);
    if !Path::new(&bin_path).exists() {
        Err(format!(
            "Terraform version {0} is not installed. Run 'terve install terraform {0}' first",
            version
        ))?
    }
    #[cfg(unix)]
    {
        let symlink_path = format!("{}/bin/terraform", dot_dir_path);
        if Path::new(&symlink_path).exists() {
            remove_file(&symlink_path)?;
        }
        std::os::unix::fs::symlink(&bin_path, &symlink_path)?;
    }
    Ok(format!("Using terraform {}", version))
}

fn do_remove_terraform_version(
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let bin_path = format!("{}/opt/terraform_{}", dot_dir_path, version);
    if Path::new(&bin_path).exists() {
        #[cfg(unix)]
        {
            let symlink_path = format!("{}/bin/terraform", dot_dir_path);
            let symlink_target_path = read_link(&symlink_path)?;
            remove_file(&bin_path)?;
            if !symlink_target_path.exists() {
                remove_file(&symlink_path)?;
            }
        }
    }
    Ok(format!("Removed terraform {}", version))
}

#[derive(FromArgs)]
/// Unified terraform and terragrunt version manager
struct Args {
    #[argh(positional)]
    action: Action,

    #[argh(positional)]
    binary: Binary,

    #[argh(positional)]
    version: Option<String>,
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
