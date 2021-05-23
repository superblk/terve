# Terve 👋

Unified, minimal [terraform](https://www.terraform.io/downloads.html) and [terragrunt](https://github.com/gruntwork-io/terragrunt/releases) version manager.

WARNING: this is in _early-ish_ development, so no releases yet. :sob:

WARNING: terraform GPG signatures are not yet verified (only sha256 validation)

## Setup

1. Build `terve` for your operating system
1. Install terve in `PATH`, e.g. in `/usr/local/bin`
1. Add directory `~/.terve/bin` to `PATH` (using e.g. `.bashrc`)

## Usage

Managed `<binary>` is `tf` (long form: `terraform`) or `tg` (long form: `terragrunt`).

Install, select and remove are idempotent, and can be run multiple times for a version without error.

List remote does not return pre-release versions (e.g. terraform `0.15.0-rc2`), but such versions can be installed/selected/removed (for testing).

### List

Lists installed (local) or available (remote) versions, sorted latest first (descending).

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

Selects a specific version for use. Said version must be installed first.

Syntax: `terve s[elect] <binary> <semver>`

- `terve s tf 0.12.31` selects terraform version 0.12.31
- `terve s tf "$(cat .terraform-version)"` selects terraform version defined in `.terraform-version`

### Remove

Removes a specific version.

Syntax: `terve r[emove] <binary> <semver>`

- `terve r tf 0.12.31` removes terraform version 0.12.31
- `terve l tf | grep 0.11. | xargs -n1 terve r tf` removes all installed terraform 0.11.x versions

## Development

You need rustup and cargo. See <https://rustup.rs/>. To run tests, run `cargo test`.

To build the binary, run `cargo build --release`. Binary is then found in `target/release/terve`.

## TODOs

- Security: implement GPG verify (terraform)
- CI: GitHub workflow release (matrix: linux + darwin)
- OS: Windows support?
