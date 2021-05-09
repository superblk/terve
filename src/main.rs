use argh::FromArgs;
use home::home_dir;
use regex::Regex;
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

// TODO: terragrunt
// TODO: implement SHA256 validation
// TODO: support macos
// TODO: tests
// TODO: -v flag (verbose)
// TODO: show which versions are installed (in list)
// TODO: support .terraform-version
// TODO: support .terragrunt-version
// TODO: implement GPG verification (terraform)

static TF_RELEASES_URL: &str = "https://releases.hashicorp.com/terraform/";

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
                    do_list_terraform_versions(version_opt).await
                }
                (Action::INSTALL, Binary::TERRAFORM, Some(version)) => {
                    do_install_terraform_version(version, dot_dir_path).await
                }
                (Action::USE, Binary::TERRAFORM, Some(version)) => {
                    do_use_terraform_version(version, dot_dir_path).await
                }
                (Action::REMOVE, Binary::TERRAFORM, Some(version)) => {
                    do_remove_terraform_version(version, dot_dir_path).await
                }
                _ => Err("invalid arguments. Run 'terve --help' for usage".into()),
            }
        }
        None => Err("unable to resolve user home directory".into()),
    }
}

fn create_dot_dir(home_dir: PathBuf) -> Result<String, Box<dyn Error>> {
    let dot_dir_path = format!("{}/.terve", home_dir.display());
    create_dir_all(format!("{}/bin", &dot_dir_path))?;
    create_dir_all(format!("{}/opt", &dot_dir_path))?;
    Ok(dot_dir_path)
}

async fn do_list_terraform_versions(version_opt: Option<String>) -> Result<String, Box<dyn Error>> {
    let releases_html = reqwest::get(TF_RELEASES_URL).await?.text().await?;
    let semver_regex = Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+").unwrap();
    let version_prefix = version_opt.unwrap_or("".to_string());
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

async fn do_install_terraform_version(
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let download_url = format!(
        "{0}{1}/terraform_{1}_linux_amd64.zip",
        TF_RELEASES_URL, version
    );
    let zip_file_bytes = reqwest::get(download_url).await?.bytes().await?;
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

async fn do_use_terraform_version(
    version: String,
    dot_dir_path: String,
) -> Result<String, Box<dyn Error>> {
    let bin_path = format!("{}/opt/terraform_{}", dot_dir_path, version);
    if !Path::new(&bin_path).exists() {
        Err(format!(
            "terraform version {0} is not installed. Run 'terve install terraform {0}' first",
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

async fn do_remove_terraform_version(
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
    USE,
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
            "u" | "use" => Ok(Action::USE),
            "r" | "remove" => Ok(Action::REMOVE),
            _ => Err(format!("action must be one of: l[ist], i[nstall], u[se], r[remove]")),
        }
    }
}

impl FromStr for Binary {
    type Err = String;

    fn from_str(a: &str) -> Result<Self, Self::Err> {
        match a {
            "tf" | "terraform" => Ok(Binary::TERRAFORM),
            "tg" | "terragrunt" => Ok(Binary::TERRAGRUNT),
            _ => Err(format!("binary must be one of: tf (alias: terraform), tg (alias: terragrunt)")),
        }
    }
}
