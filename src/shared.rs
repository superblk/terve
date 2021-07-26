use std::{
    error::Error,
    fmt::Display,
    fs::{create_dir_all, hard_link, read_dir, remove_file},
    path::{Path, PathBuf},
    str::FromStr,
};

use same_file::is_same_file;
use semver::{Prerelease, Version};
use std::env::consts::EXE_SUFFIX;
use crate::utils;

pub enum Action {
    List,
    Install,
    Select,
    Remove,
    Which,
}

pub enum Binary {
    Terraform,
    Terragrunt,
}

impl FromStr for Action {
    type Err = String;

    fn from_str(a: &str) -> Result<Self, Self::Err> {
        match a {
            "l" | "list" => Ok(Action::List),
            "i" | "install" => Ok(Action::Install),
            "s" | "select" => Ok(Action::Select),
            "r" | "remove" => Ok(Action::Remove),
            "w" | "which" => Ok(Action::Which),
            _ => Err(
                "Action must be one of: l[ist], i[nstall], s[elect], r[emove] or w[hich]"
                    .to_string(),
            ),
        }
    }
}

impl FromStr for Binary {
    type Err = String;

    fn from_str(a: &str) -> Result<Self, Self::Err> {
        match a {
            "tf" | "terraform" => Ok(Binary::Terraform),
            "tg" | "terragrunt" => Ok(Binary::Terragrunt),
            _ => Err("Binary must be one of: tf, tg, terraform or terragrunt".to_string()),
        }
    }
}

impl Display for Binary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Binary::Terraform => write!(f, "terraform"),
            Binary::Terragrunt => write!(f, "terragrunt"),
        }
    }
}

impl AsRef<Path> for Binary {
    fn as_ref(&self) -> &Path {
        let path = match *self {
            Binary::Terraform => {
                if cfg!(unix) {
                    "terraform"
                } else {
                    "terraform.exe"
                }
            }
            Binary::Terragrunt => {
                if cfg!(unix) {
                    "terragrunt"
                } else {
                    "terragrunt.exe"
                }
            }
        };
        Path::new(path)
    }
}

pub struct DotDir {
    pub root: PathBuf,
    pub bin: PathBuf,
    pub etc: PathBuf,
    pub opt: PathBuf,
}

impl DotDir {
    pub fn bootstrap(home_dir: &Path) -> Result<DotDir, Box<dyn Error>> {
        let root = home_dir.join(".terve");
        let bin = root.join("bin");
        let etc = root.join("etc");
        let opt = root.join("opt");
        create_dir_all(&bin)?;
        create_dir_all(&etc)?;
        create_dir_all(opt.join(Binary::Terraform))?;
        create_dir_all(opt.join(Binary::Terragrunt))?;
        Ok(DotDir {
            root,
            bin,
            etc,
            opt,
        })
    }
}

pub fn list_available_versions(git_repo_url: &str) -> Result<String, Box<dyn Error>> {
    let mut versions: Vec<Version> = utils::git_list_remote_tags(git_repo_url)?
        .iter()
        .map(|t| t.trim_start_matches('v'))
        .filter_map(|s| Version::parse(s).ok())
        .filter(|v| v.pre == Prerelease::EMPTY)
        .collect();
    let result = utils::to_sorted_multiline_string(&mut versions);
    Ok(result)
}

pub fn list_installed_versions(binary: Binary, dot_dir: DotDir) -> Result<String, Box<dyn Error>> {
    let opt_dir = dot_dir.opt.join(binary);
    let mut installed_versions: Vec<Version> = read_dir(&opt_dir)?
        .filter_map(|r| Some(r.ok()?.path()))
        .filter_map(|p| Some(p.strip_prefix(&opt_dir).ok()?.to_owned()))
        .filter_map(|p| Version::parse(p.to_string_lossy().as_ref()).ok())
        .collect();
    let result = utils::to_sorted_multiline_string(&mut installed_versions);
    Ok(result)
}

pub fn select_binary_version(
    binary: Binary,
    version: String,
    dot_dir: DotDir,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(&binary).join(&version);
    if !opt_file_path.exists() {
        return Err(format!(
            "{0} version {1} is not installed. Run 'terve install {0} {1}'",
            binary, version
        )
        .into());
    }
    let bin_file_path = dot_dir.bin.join(&binary);
    if bin_file_path.exists() {
        remove_file(&bin_file_path)?;
    }
    hard_link(&opt_file_path, &bin_file_path)?;
    Ok(format!("Selected {} {}", binary, version))
}

pub fn remove_binary_version(
    binary: Binary,
    version: String,
    dot_dir: DotDir,
) -> Result<String, Box<dyn Error>> {
    let opt_file_path = dot_dir.opt.join(&binary).join(&version);
    if opt_file_path.exists() {
        let bin_file_path = dot_dir.bin.join(&binary);
        if is_same_file(&opt_file_path, &bin_file_path)? {
            return Err(format!(
                "{0} version {1} is currently selected. Use 'terve select {0} <version>' to select another version first",
                &binary, version
            )
            .into());
        }
        remove_file(&opt_file_path)?;
    }
    Ok(format!("Removed {} {}", binary, version))
}

pub fn get_selected_version(binary: Binary, dot_dir: DotDir) -> Result<String, Box<dyn Error>> {
    let bin_file_path = dot_dir.bin.join(&binary);
    let result = if bin_file_path.exists() {
        let opt_dir_path = dot_dir.opt.join(&binary);
        find_binary_version(bin_file_path, opt_dir_path)?
    } else {
        "".to_string()
    };
    Ok(result)
}

fn find_binary_version(
    bin_file_path: PathBuf,
    opt_dir_path: PathBuf,
) -> Result<String, Box<dyn Error>> {
    let expected_file_len = bin_file_path.metadata()?.len();
    let candidates: Vec<PathBuf> = read_dir(&opt_dir_path)?
        .filter_map(|r| Some(r.ok()?.path()))
        .filter(|p| p.metadata().is_ok())
        .filter(|p| p.metadata().unwrap().len() == expected_file_len)
        .collect();
    for opt_file_path in candidates {
        if is_same_file(&bin_file_path, &opt_file_path)? {
            if let Some(f) = &opt_file_path.file_name() {
                let result = f.to_string_lossy().to_string().strip_suffix(EXE_SUFFIX);
                return Ok(result);
            }
        }
    }
    Ok("".to_string())
}
