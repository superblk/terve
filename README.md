# Terve 👋

![Release](https://img.shields.io/github/v/release/superblk/terve)
![License](https://img.shields.io/github/license/superblk/terve)
![OS](https://img.shields.io/badge/os-Linux%20%7C%20macOS%20%7C%20Windows-ff69b4)

Unified, minimal [terraform](https://www.terraform.io/downloads.html) and [terragrunt](https://github.com/gruntwork-io/terragrunt/releases) version manager.

## Features

- Minimal by design: no shims, no magic, quiet, but extendable thru scripting
- SHA256 checksums are checked for terraform and terragrunt binary downloads
- PGP signatures are checked for terraform binary downloads (if Hashicorp's public key is configured)

## Supported platforms

- Linux (amd64, arm64)
    - NOTE: only terraform `0.11.15`, `0.12.{30,31}`, `0.13.5+` and terragrunt `0.28.12+` ship linux arm64 binaries
- macOS (amd64, arm64)
    - NOTE: only terraform `1.1.0+` and terragrunt `0.28.12+` ship macOS arm64 binaries
- Windows (amd64)

⚠️ A pre-built terve binary is not yet available for macOS arm64, because Apple M1-based GitHub runners are not yet available, see this [issue](https://github.com/actions/virtual-environments/issues/2187)

## Setup

1. [Download](https://github.com/superblk/terve/releases/latest) terve for your platform, check file integrity, and install the binary somewhere in your `PATH`, e.g. `/usr/local/bin/terve`
    - On Linux/macOS, file integrity can be checked in the directory where you downloaded the binary and `SHA256SUMS`, like so:
           
           $ ls
           SHA256SUMS terve_linux_amd64
           $ sha256sum -c --ignore-missing 2>/dev/null SHA256SUMS
           terve_linux_amd64: OK
    
    - On Linux/macOS, be sure to make the binary executable: `chmod +x terve`
1. Create the `~/.terve` directory tree by running `terve --bootstrap`
1. Add the `~/.terve/bin` directory to `PATH` (using e.g. `.bashrc` or Windows' control panel)
1. Copy Hashicorp's [PGP public key](https://www.hashicorp.com/security) in `~/.terve/etc/terraform.asc` (read-only, mode `0444` on Linux/macOS)
    - This public key is used to verify terraform binary download PGP signatures
    - If not installed (or bad file permissions), terve will log a warning for terraform installs
1. [Install your desired versions of terraform and terragrunt](#install)
1. [Select your desired versions of terraform and terragrunt](#select)

## How it works

Terve uses **hard** links to configure selected terraform/terragrunt binary versions.

All files are kept in directory `~/.terve` like so (example directory tree for Linux):

```txt
/home/whoami/.terve
├── bin
│   ├── terraform
│   └── terragrunt
├── etc
│   └── terraform.asc
├── opt
│   ├── terraform
│   │   ├── 0.1.0
│   │   ├── 1.0.2
│   │   └── 1.0.3
│   └── terragrunt
│       ├── 0.0.4
│       ├── 0.1.0
│       └── 0.31.2
└── var
    ├── terraform
    │   └── version
    └── terragrunt
        └── version
```

- `bin` – _selected_ terraform and terragrunt binaries (hard-linked to files in `opt`)
- `etc` – configuration
- `opt` – _installed_ terraform and terragrunt binaries, version is encoded in file name
- `var` – variable data, currently only holders for selected version strings

## Usage

Managed `<binary>` is `tf` (long form: `terraform`) or `tg` (long form: `terragrunt`).

Install, select and remove are idempotent, and can be run multiple times for a version without error.

### List

Lists installed or available (remote) versions, sorted latest first.

Syntax: `terve l[ist] <binary> [spec]` where `spec` is `r[emote]`

- `terve l tf` lists installed (local) terraform versions
- `terve l tf r` lists available (remote) terraform versions
- `terve l tf r | tac` lists available terraform versions, _sorted oldest first_
- `terve l tg r | grep 0.29.` lists available terragrunt 0.29.x versions

💡 List remote does not return pre-release versions (e.g. terraform `0.15.0-rc2`), but such versions can be installed/selected/removed (for testing).

### Install

Installs a specific version.

Syntax: `terve i[nstall] <binary> <semver>`

- `terve i tf 0.12.31` installs terraform version 0.12.31
- `terve i tf "$(terve l tf r | head -n1)"` installs latest version of terraform
- `terve i tf "$(cat .terraform-version)"` installs terraform version defined in `.terraform-version`
- `terve i tg "$(cat .terragrunt-version)"` installs terragrunt version defined in `.terragrunt-version`
- `terve l tg r | grep 0.29. | xargs -n1 -P4 terve i tg` installs all available terragrunt 0.29.x versions

⚠️ terragrunt releases < `0.18.1` do not ship `SHA256SUMS` files, so their file integrity cannot be checked

### Select

Selects an installed version for use.

Syntax: `terve s[elect] <binary> <semver>`

- `terve s tf 0.12.31` selects terraform version 0.12.31
- `terve s tf "$(cat .terraform-version)"` selects terraform version defined in `.terraform-version`

### Remove

Removes an installed version.

Syntax: `terve r[emove] <binary> <semver>`

- `terve r tf 0.12.31` removes terraform version 0.12.31
- `terve l tf | grep 0.11. | xargs -n1 terve r tf` removes all installed terraform 0.11.x versions

💡 Removing a version does not reset current selection

### Which

Tells which version is currently selected.

Syntax: `terve w[hich] <binary>`

```shell
$ terve w tf
0.15.5
```

## Optional shell extensions

### Terraform switch (Linux and macOS)

Simple switch to specified terraform version.

```sh
function tfv {
  local version="$1"
  if [ -z "$version" ]; then
    echo "Usage: tfv <version>, e.g. tfv 1.2.3"
    return 1
  fi
  terve i tf "$version" >/dev/null && \
  terve s tf "$version" >/dev/null && \
  echo "Switched to terraform $version"
}
```

### Terra(form|grunt) use (Linux and macOS)

Use terraform and terragrunt versions defined in `.terraform-version` and `.terragrunt-version` (in current working directory or any parent directory)

```sh
#!/bin/sh

upcat() {
    file="$1"
    while [ "$PWD" != "/" ]; do
        if [ -r "$file" ]; then
            cat "$file"
            break
        fi
        cd ..
    done
}

tf_version="$(upcat .terraform-version)"
tg_version="$(upcat .terragrunt-version)"

if [ -z "$tf_version" ]; then
    echo "ERROR: No .terraform-version file found"
    exit 1
fi

if [ -z "$tg_version" ]; then
    echo "ERROR: No .terragrunt-version file found"
    exit 2
fi

terve i tf "$tf_version" && terve s tf "$tf_version"
terve i tg "$tg_version" && terve s tg "$tg_version"
```

## Development

You need [cargo](https://rustup.rs/) (Rust's build tool). To run all tests, run `cargo test`.

Visual Studio Code with [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer) provides a good IDE experience.

To build a release binary, run `cargo build --release`. Binary is then found in `target/release/`.
