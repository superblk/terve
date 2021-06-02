# Terve 👋

Unified, minimal [terraform](https://www.terraform.io/downloads.html) and [terragrunt](https://github.com/gruntwork-io/terragrunt/releases) version manager.

WARNING: this is a new project, and is very subject to change

## Supported platforms

- Linux (amd64)
- MacOS (amd64)
- Windows (amd64)

## Setup

1. [Build](https://github.com/superblk/terve#development), and install `terve` in `PATH`, e.g. in `/usr/local/bin`
1. Add the directory `~/.terve/bin` to `PATH` (using e.g. `.bashrc`)
1. Create the `~/.terve` directory tree by running `terve --bootstrap`
1. Install Hashicorp's [PGP public key](https://www.hashicorp.com/security) in `~/.terve/etc/terraform.asc` (mode `0444`)
    - This public key is used to verify terraform download PGP signatures
    - If not installed (or bad file permissions), terve will log a warning for each terraform install

## Layout

Terve keeps files in directory `$HOME/.terve` like so:

```txt
/home/whoami/.terve
├── bin
│   ├── terraform -> /home/whoami/.terve/opt/terraform/0.15.4
│   └── terragrunt -> /home/whoami/.terve/opt/terragrunt/0.28.10
├── etc
│   └── terraform.asc
└── opt
    ├── terraform
    │   ├── 0.14.11
    │   └── 0.15.4
    └── terragrunt
        ├── 0.28.10
        ├── 0.28.39
        └── 0.29.4
```

## Usage

Managed `<binary>` is `tf` (long form: `terraform`) or `tg` (long form: `terragrunt`).

Install, select and remove are idempotent, and can be run multiple times for a version without error.

List remote does not return pre-release versions (e.g. terraform `0.15.0-rc2`), but such versions can be installed/selected/removed (for testing).

### List

Lists installed or available (remote) versions, sorted latest first (descending).

Syntax: `terve l[ist] <binary> [spec]` where `spec` is `r[emote]`

- `terve l tf` lists installed (local) terraform versions
- `terve l tf r` lists available (remote) terraform versions
- `terve l tg r | grep 0.29.` lists available terragrunt 0.29.x versions

### Install

Installs a specific version.

Syntax: `terve i[nstall] <binary> <semver>`

- `terve i tf 0.12.31` installs terraform version 0.12.31
- `terve i tf "$(terve l tf r | head -n1)"` installs latest version of terraform
- `terve i tf "$(cat .terraform-version)"` installs terraform version defined in `.terraform-version`
- `terve i tg "$(cat .terragrunt-version)"` installs terragrunt version defined in `.terragrunt-version`
- `terve l tg r | grep 0.29. | xargs -n1 -P4 terve i tg` installs all available terragrunt 0.29.x versions

WARNING: terragrunt releases < `0.18.1` do not ship `SHA256SUMS` files, so their file integrity cannot be checked

### Select

Selects a specific version for use. That version must be installed first.

Syntax: `terve s[elect] <binary> <semver>`

- `terve s tf 0.12.31` selects terraform version 0.12.31
- `terve s tf "$(cat .terraform-version)"` selects terraform version defined in `.terraform-version`

### Remove

Removes a specific version.

Syntax: `terve r[emove] <binary> <semver>`

- `terve r tf 0.12.31` removes terraform version 0.12.31
- `terve l tf | grep 0.11. | xargs -n1 terve r tf` removes all installed terraform 0.11.x versions

## Development

You need [cargo](https://rustup.rs/) and [OpenSSL pre-requisites](https://docs.rs/openssl#automatic). To run all tests, run `cargo test`.

To build the binary, run `cargo build --release`. Binary is then found in `target/release/terve`.

Visual Studio Code with [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer) provides a good IDE experience.

## TODOs

- QA: Improve test coverage
- CI: Release workflow (matrix: linux + darwin)
- Err: more contextual error messages (anyhow?)
