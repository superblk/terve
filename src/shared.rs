use std::{
    error::Error,
    fmt::Display,
    fs::{create_dir_all, read_dir, read_link, remove_file},
    path::{Path, PathBuf},
    str::FromStr,
};

use semver::Version;

use crate::utils;

pub enum Action {
    List,
    Install,
    Select,
    Remove,
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
            _ => Err("Action must be one of: l[ist], i[nstall], s[elect] or r[emove]".to_string()),
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
    let symlink_path = dot_dir.bin.join(&binary);
    let opt_file_path = dot_dir.opt.join(&binary).join(&version);
    if !Path::new(&opt_file_path).exists() {
        return Err(format!(
            "{0} version {1} is not installed. Run 'terve install {0} {1}'",
            binary, version
        )
        .into());
    }
    if read_link(&symlink_path).is_ok() {
        remove_file(&symlink_path)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&opt_file_path, &symlink_path)?;
    }
    #[cfg(windows)]
    {
        use std::fs::copy;
        use std::os::windows::fs::symlink_file;
        let copy_binary = |_| {
            eprint!("WARNING: Unable to create symlink, copying binary instead. See https://github.com/superblk/terve#how-it-works{}", utils::NEWLINE);
            copy(&opt_file_path, &symlink_path)
        };
        symlink_file(&opt_file_path, &symlink_path).or_else(copy_binary)?;
    }
    Ok(format!("Selected {} {}", binary, version))
}

pub fn remove_binary_version(
    binary: Binary,
    version: String,
    dot_dir: DotDir,
) -> Result<String, Box<dyn Error>> {
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
