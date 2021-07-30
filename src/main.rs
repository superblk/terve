use dirs::home_dir;
use pico_args::Arguments;
use semver::Version;
use shared::{Action, Binary, DotDir};
use std::{
    env::consts::{ARCH, OS},
    str::FromStr,
};
use std::{error::Error, process};
use terraform::TF_GIT_REPOSITORY_URL;
use terragrunt::TG_GIT_REPOSITORY_URL;
use utils::{eprintln, println};

mod http;
mod shared;
mod terraform;
mod terragrunt;
mod utils;

fn main() {
    process::exit(match run() {
        Ok(s) => {
            if !s.is_empty() {
                println(&s);
            }
            0
        }
        Err(e) => {
            eprintln(e);
            1
        }
    });
}

fn run() -> Result<String, Box<dyn Error>> {
    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        return Ok(USAGE_HELP_MSG.to_string());
    }

    if args.contains(["-v", "--version"]) {
        return Ok(TERVE_VERSION.to_string());
    }

    if let Some(home) = home_dir() {
        let dot_dir = DotDir::bootstrap(&home)?;

        if args.contains(["-b", "--bootstrap"]) {
            return Ok(format!("Created {}", dot_dir.root.display()));
        }

        let (action, binary, version, os, arch) = get_params(args)?;

        match (action, binary, version) {
            (Action::List, binary, None) => shared::list_installed_versions(binary, dot_dir),
            (Action::List, Binary::Terraform, Some(v)) if v.is_remote() => {
                shared::list_available_versions(TF_GIT_REPOSITORY_URL)
            }
            (Action::List, Binary::Terragrunt, Some(v)) if v.is_remote() => {
                shared::list_available_versions(TG_GIT_REPOSITORY_URL)
            }
            (Action::Install, Binary::Terraform, Some(v)) if v.is_semver() => {
                terraform::install_binary_version(v, dot_dir, os, arch)
            }
            (Action::Install, Binary::Terragrunt, Some(v)) if v.is_semver() => {
                terragrunt::install_binary_version(v, dot_dir, os, arch)
            }
            (Action::Select, binary, Some(v)) if v.is_semver() => {
                shared::select_binary_version(binary, v, dot_dir)
            }
            (Action::Remove, binary, Some(v)) if v.is_semver() => {
                shared::remove_binary_version(binary, v, dot_dir)
            }
            (Action::Which, binary, None) => shared::get_selected_version(binary, dot_dir),
            _ => Err(INVALID_ARGS_MSG.into()),
        }
    } else {
        Err("Unable to resolve user home directory".into())
    }
}

type Params = (Action, Binary, Option<String>, String, String);

fn get_params(mut args: Arguments) -> Result<Params, Box<dyn Error>> {
    let action: Action = match args.subcommand()? {
        Some(s) => Action::from_str(&s)?,
        None => return Err(INVALID_ARGS_MSG.into()),
    };

    let binary: Binary = match args.subcommand()? {
        Some(s) => Binary::from_str(&s)?,
        None => return Err(INVALID_ARGS_MSG.into()),
    };

    let version: Option<String> = args.subcommand()?;

    let os = match OS {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "windows",
        other => panic!("Unsupported OS: {}", other),
    };

    let arch = match ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => panic!("Unsupported architecture: {}", other),
    };

    Ok((action, binary, version, os.to_string(), arch.to_string()))
}

trait VersionQualifier {
    fn is_remote(&self) -> bool;
    fn is_semver(&self) -> bool;
}

impl VersionQualifier for String {
    fn is_remote(&self) -> bool {
        self == "r" || self == "remote"
    }

    fn is_semver(&self) -> bool {
        Version::parse(self).is_ok()
    }
}

const TERVE_VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE_HELP_MSG: &str = "\
Unified terraform and terragrunt version manager

See https://github.com/superblk/terve for documentation

USAGE:
  terve <ACTION> <BINARY> [<VERSION>]

ACTION:
  l, list               Lists versions
  i, install            Installs given version
  s, select             Selects installed version
  r, remove             Removes installed version
  w, which              Prints selected version

BINARY:
  tf, terraform         Terraform (https://www.terraform.io/)
  tg, terragrunt        Terragrunt (https://terragrunt.gruntwork.io/)

VERSION:
  r, remote             Available (remote) versions
  x.y.z                 Semantic version string, e.g. 0.15.4

FLAGS:
  -h, --help            Prints this help message
  -v, --version         Prints application version
  -b, --bootstrap       Creates ~/.terve directory tree

EXAMPLES:
  terve l tf            Lists installed terraform versions
  terve l tf r          Lists available terraform versions
  terve i tf 0.15.4     Installs terraform 0.15.4
  terve s tf 0.15.4     Selects terraform 0.15.4
  terve r tf 0.15.4     Removes terraform 0.15.4
  terve w tf            Prints selected terraform version
";

const INVALID_ARGS_MSG: &str = "Invalid arguments. Run 'terve --help' for usage";
