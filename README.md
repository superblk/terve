# Terve

Unified terraform and terragrunt version manager.

## Setup

1. Download terve for your operating system
1. Install terve in `PATH`, e.g. in `/usr/local/bin`
1. Add directory `~/.terve/bin` to `PATH` (using e.g. `.bashrc`)

## Usage

NOTE: `<binary>` is either `tf` (`terraform`) or `tg` (`terragrunt`)

### List

Syntax: `terve l[ist] <binary> [spec]`

- `terve l tf` lists installed terraform versions
- `terve l tg` lists installed terragrunt versions
- `terve l tf remote` lists available terraform versions
- `terve l tg remote | grep 0.29.` lists available terragrunt 0.29.x versions

### Install

Syntax: `terve i[nstall] <binary> <semver>`

- `terve i tf 0.12.31` installs terraform version 0.12.31
- `terve i tf "$(cat .terraform-version)"` installs terraform version defined in `.terraform-version`
- `terve i tg "$(cat .terragrunt-version)"` installs terragrunt version defined in `.terragrunt-version`
- `terve l tg remote | grep 0.29. | xargs -n1 -P4 terve i tg` installs all terragrunt 0.29.x versions

### Select

Syntax: `terve s[elect] <binary> <semver>`

- `terve s tf 0.12.31` selects terraform version 0.12.31
- `terve s tf "$(cat .terraform-version)"` selects terraform version defined in `.terraform-version`

NOTE: selected version must be installed first

### Remove

Syntax: `terve r[emove] <binary> <semver>`

- `terve r tf 0.12.31` removes terraform version 0.12.31
- `terve l tf | grep 0.11. | xargs -n1 -P4 terve r tf` removes all installed terraform 0.11.x versions

NOTE: remove does not fail if given version is not installed

## TODOs

- GH workflow build
- support macos
- tests
- implement GPG verify (terraform)
