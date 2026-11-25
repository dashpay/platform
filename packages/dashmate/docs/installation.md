# Installation

This guide provides instructions for installing Dashmate, a distribution package for Dash node installation.
The guide is written for Ubuntu 22.04 x64 LTS, but the steps should be similar for other Linux distributions.

## Install dependencies

Install and configure Docker:

```bash
curl -fsSL https://get.docker.com -o get-docker.sh && sh ./get-docker.sh
sudo usermod -aG docker $USER
newgrp docker
```

## Install dashmate

There are several methods available for installing dashmate.
Installing the Linux, MacOS, or Windows packages from the [GitHub releases page](https://github.com/dashpay/platform/releases/latest) is recommended for mainnet masternodes.

### Debian package

Download the newest dashmate installation package for your architecture from the [GitHub releases page](https://github.com/dashpay/platform/releases/latest).
The file name contains the version, so it changes with every release; this downloads the one matching the architecture you are on:

```bash
DASHMATE_DEB="$(curl -fsSL https://api.github.com/repos/dashpay/platform/releases/latest \
  | grep -o "https://[^\"]*_$(dpkg --print-architecture)\.deb" \
  | head -n 1)"
if [ -z "$DASHMATE_DEB" ]; then
  echo "Could not find a dashmate .deb for this architecture in the latest release" >&2
else
  curl -fLO "$DASHMATE_DEB"
fi
```

Install the package that was just downloaded:

```bash
sudo apt update
sudo apt install "./$(basename "$DASHMATE_DEB")"
```

> **Note:** At the end of the installation process, apt may display an error due to installing a downloaded package.
> You can ignore this error message:
> N: Download is performed unsandboxed as root as file '/home/ubuntu/dashmate_5.1.0-1_amd64.deb' couldn't be accessed by user '_apt'. - pkgAcquire::Run (13: Permission denied)

#### Dash's apt repository

The package configures Dash's apt repository, `https://apt.services.dash.org/dashmate`, so later releases install with `sudo apt update && sudo apt install dashmate`. It ships three files:

- `/etc/apt/sources.list.d/dashmate.sources`, the repository source. It names the `stable` suite, which carries final releases only. To follow prereleases as well, change `Suites: stable` to `Suites: testing` in this file. Upgrades keep the change without asking, since the package's copy of the file stays the same.
- `/etc/apt/keyrings/dashmate-archive-keyring.gpg`, the key the repository is signed with. An unmodified copy is replaced on upgrade, which is how a renewed key arrives.
- `/etc/apt/preferences.d/dashmate.pref`, which limits the repository to the dashmate package: apt does not choose any other package from it as a candidate, in any architecture. This does not make it safe for the repository to list other packages. If it lists a package at the same version as the distribution with matching package metadata (the same control fields and size), apt can treat the two as one version even when their contents differ, and the pin then hides the distribution's copy too, including a security update. The repository's publisher accepts only the dashmate package, and that check is what keeps other packages out.

`apt remove dashmate` keeps the three files and `apt purge dashmate` deletes them. Once dashmate is removed, upgrades no longer renew the key, so when the key expires, `apt update` reports a signature error for the repository. To take dashmate off a host for good, purge it. To keep the source, with its suite choice, without using it, add `Enabled: no` to `dashmate.sources`; a later install keeps that line, so delete it to use the repository again.

Do not create these files yourself, and do not create others with the same names. If one of them already exists with different content when the package is installed, dpkg asks whether to keep it. An install with no terminal to answer that question, such as an `apt-get -y` run from a script, fails at the prompt and leaves dashmate unconfigured. A repository configured by hand, for example to install a prerelease that does not ship these files yet, belongs in files of other names. Create the pin first, before the first `apt update` that reads the source, and check the key before using it: the file's sha256 is `08006df56f24e282f19012a29e539dad431654abefefd697832ac61918b01230` and its primary key fingerprint is `A5F90926475EF25346DCE26836EF5BF853D100FE`.

```bash
sudo install -d -m 0755 /etc/apt/keyrings /etc/apt/preferences.d
sudo tee /etc/apt/preferences.d/dashmate-testing.pref > /dev/null <<'EOF'
Package: dashmate:any
Pin: origin apt.services.dash.org
Pin-Priority: 500

Package: *:any
Pin: origin apt.services.dash.org
Pin-Priority: -1
EOF
sudo curl -fsSL https://apt.services.dash.org/dashmate/dashmate-archive-keyring.gpg \
  -o /etc/apt/keyrings/dashmate-testing-archive-keyring.gpg
sha256sum /etc/apt/keyrings/dashmate-testing-archive-keyring.gpg
sudo tee /etc/apt/sources.list.d/dashmate-testing.sources > /dev/null <<'EOF'
Types: deb
URIs: https://apt.services.dash.org/dashmate
Suites: testing
Components: main
Architectures: amd64 arm64
Signed-By: /etc/apt/keyrings/dashmate-testing-archive-keyring.gpg
EOF
sudo apt update
sudo apt install dashmate
```

Use `Suites: dev` for dev builds.

Once dashmate is installed from a package that ships `dashmate.sources`, delete those three hand-written files first, then set `Suites:` in `dashmate.sources`: while both sources name the same suite with different keys, `apt update` fails with `Conflicting values set for option Signed-By`.

```bash
sudo rm /etc/apt/sources.list.d/dashmate-testing.sources \
  /etc/apt/keyrings/dashmate-testing-archive-keyring.gpg \
  /etc/apt/preferences.d/dashmate-testing.pref
sudo sed -i 's/^Suites: stable$/Suites: testing/' /etc/apt/sources.list.d/dashmate.sources
sudo apt update
```

dpkg does not own the hand-written files, so purging dashmate leaves them in place and the source active. To take dashmate off a host that still uses them, delete the three files with the `rm` command above. To keep the suite choice without an active source, add `Enabled: no` to `dashmate-testing.sources` instead.

### Node package

> **Warning:** This installation option is not recommended for mainnet masternodes.
> Please install packages from the GitHub releases page.

To install the NPM package, it is necessary to install Node.JS first. We recommend installing it using [nvm](https://github.com/nvm-sh/nvm#readme):

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.5/install.sh | bash
source ~/.bashrc
nvm install 24
```

Once Node.JS has been installed, use NPM to install dashmate:

```bash
npm install -g dashmate
```

## Verifying Installation

After installation, verify that Dashmate is correctly installed by running:

```bash
$ dashmate --version
```

You should see the version number of Dashmate displayed.

## Next Steps

After installing Dashmate, you're ready to:

1. [Set up your Dash node](./commands/setup.md)
2. [Configure your node](./commands/config/index.md)
3. [Start your node](./commands/start.md)

For updating Dashmate to a newer version, refer to the [Update Guide](./update.md).
