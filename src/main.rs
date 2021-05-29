use home::home_dir;
use pico_args::Arguments;
use semver::Version;
use shared::{Action, Binary, DotDir};
use std::{env::consts::OS, str::FromStr};
use std::{error::Error, process};

mod http;
mod shared;
mod terraform;
mod terragrunt;
mod utils;

const TERVE_VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE_HELP_MSG: &str = "\
Unified terraform and terragrunt version manager

See https://github.com/superblk/terve for documentation

USAGE:
  terve <ACTION> <BINARY> [<VERSION>]

ACTION:
  l, list               Lists versions
  i, install            Installs a version
  s, select             Selects an installed version
  r, remove             Removes an installed version

BINARY:
  tf, terraform         Terraform (https://www.terraform.io/)
  tg, terragrunt        Terragrunt (https://terragrunt.gruntwork.io/)

VERSION:
  r, remote             Available versions (list only)
  x.y.z                 Semantic version string, e.g. 0.15.4

FLAGS:
  -h, --help            Prints this help message
  -v, --version         Prints application version
  -b, --bootstrap       Bootstraps ~/.terve directory tree
";

const INVALID_ARGS_MSG: &str = "Invalid arguments. Run 'terve --help' for usage help";

fn main() {
    process::exit(match run() {
        Ok(s) => {
            if !s.is_empty() {
                println!("{}", s);
            }
            0
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
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
            return Ok(format!("Created {}/.terve", home.display()));
        }

        let (action, binary, version, os) = get_params(args)?;

        match (action, binary, version, os) {
            (Action::List, binary, None, _) => shared::list_installed_versions(binary, dot_dir),
            (Action::List, Binary::Terraform, Some(v), _) if v.is_remote() => {
                terraform::list_available_versions()
            }
            (Action::List, Binary::Terragrunt, Some(v), _) if v.is_remote() => {
                terragrunt::list_available_versions()
            }
            (Action::Install, Binary::Terraform, Some(v), os) if v.is_semver() => {
                terraform::install_binary_version(v, dot_dir, os)
            }
            (Action::Install, Binary::Terragrunt, Some(v), os) if v.is_semver() => {
                terragrunt::install_binary_version(v, dot_dir, os)
            }
            (Action::Select, binary, Some(v), _) if v.is_semver() => {
                shared::select_binary_version(binary, v, dot_dir)
            }
            (Action::Remove, binary, Some(v), _) if v.is_semver() => {
                shared::remove_binary_version(binary, v, dot_dir)
            }
            _ => Err(INVALID_ARGS_MSG.into()),
        }
    } else {
        Err("Unable to resolve user home directory (HOME unset?)".into())
    }
}

type Params = (Action, Binary, Option<String>, String);

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
        "linux" => "linux".to_string(),
        "macos" => "darwin".to_string(),
        os => panic!("Unsupported OS: {}", os),
    };

    Ok((action, binary, version, os))
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
