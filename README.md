# Terve 👋

Unified, minimal [terraform](https://www.terraform.io/downloads.html) and [terragrunt](https://github.com/gruntwork-io/terragrunt/releases) version manager.

WARNING: this is a new project and is subject to change, hence no releases yet

## Supported platforms

- Linux (amd64)
- MacOS (amd64)

## Setup

1. Install `terve` in `PATH`, e.g. in `/usr/local/bin`
1. Add directory `~/.terve/bin` to `PATH` (using e.g. `.bashrc`)
1. Create the `~/.terve` directory tree by running `terve --bootstrap`
1. Install Hashicorp's [PGP public key](https://www.hashicorp.com/security) in `~/.terve/etc/terraform.asc` (mode `0444`)
    - This key is used to validate terraform download PGP signatures
    - If not present, terve will log a warning for each terraform install

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

WARNING: list remote for terragrunt uses GitHub API which is rate-limited (GitHub API throws 403 Forbidden if rate-limit quota is depleted)!

### Install

Installs a specific version.

Syntax: `terve i[nstall] <binary> <semver>`

- `terve i tf 0.12.31` installs terraform version 0.12.31
- `terve i tf "$(terve l tf r | head -n1)"` installs latest version of terraform
- `terve i tf "$(cat .terraform-version)"` installs terraform version defined in `.terraform-version`
- `terve i tg "$(cat .terragrunt-version)"` installs terragrunt version defined in `.terragrunt-version`
- `terve l tg r | grep 0.29. | xargs -n1 -P4 terve i tg` installs all available terragrunt 0.29.x versions

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

## Examples

```bash
# CI automation example

tf_version="$(cat .terraform-version 2>/dev/null || echo 0.15.4)"
tg_version="$(cat .terragrunt-version 2>/dev/null || echo 0.29.4)"

terve i tf "$tf_version" && terve s tf "$tf_version"
terve i tg "$tg_version" && terve s tg "$tg_version"

terragrunt plan
```

## Development

You need rustup and cargo. See <https://rustup.rs/>. To run all tests, run `cargo test`.

To build the binary, run `cargo build --release`. Binary is then found in `target/release/terve`.

## TODOs

- CI: Release workflow (matrix: linux + darwin)
- OS: Windows support?
