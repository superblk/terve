# Terve

Unified terraform and terragrunt version manager.

## Setup

1. Download terve for your operating system
1. Install terve in `PATH`, e.g. in `/usr/local/bin`
1. Add the directory `~/.terve/bin` to `PATH`
1. Run `terve list terraform 0.15` to test

## Usage

### List

Syntax: `terve l[ist] <binary> [spec]`

- `terve list terraform` lists all available terraform versions
- `terve l tf 0.12` lists all available terraform 0.12.x versions 

### Install

Syntax: `terve i[nstall] <binary> <version>`

- `terve install terraform 0.12.31` installs terraform 0.12.31
- `terve i tf "$(cat .terraform-version)"` installs the terraform version from `.terraform-version`

### Select

NOTE: selected version must be installed first

Syntax: `terve s[elect] <binary> <version>`

- `terve select terraform 0.12.31` selects terraform 0.12.31 into use
- `terve s tf "$(cat .terraform-version)"` selects terraform version from `.terraform-version` into use

### Remove

Syntax: `terve r[emove] <binary> <version>`

- `terve remove terraform 0.12.31` removes terraform 0.12.31
- `terve r tf 0.15.3` removes terraform 0.15.3
