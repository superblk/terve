use std::{
    error::Error,
    fmt::Display,
    fs::{create_dir_all, read_dir, read_link, remove_file},
    path::{Path, PathBuf},
    str::FromStr,
};

use semver::Version;

pub enum Action {
    LIST,
    INSTALL,
    SELECT,
    REMOVE,
}

pub enum Binary {
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
                "action must be one of: l[ist], i[nstall], s[elect] or r[emove]"
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
                "binary must be one of: tf, tg, terraform or terragrunt"
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

pub struct DotDir {
    pub bin: PathBuf,
    pub opt: PathBuf,
}

impl DotDir {
    pub fn init(home_dir: PathBuf) -> Result<DotDir, Box<dyn Error>> {
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

pub fn list_installed_versions(binary: Binary, dot_dir: DotDir) -> Result<String, Box<dyn Error>> {
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

pub fn select_binary_version(
    binary: Binary,
    version: String,
    dot_dir: DotDir,
) -> Result<String, Box<dyn Error>> {
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