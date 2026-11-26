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
On Debian and Ubuntu, installing it from Dash's apt repository is recommended for mainnet masternodes; on other systems, install the Linux, MacOS, or Windows packages from the [GitHub releases page](https://github.com/dashpay/platform/releases/latest).

### Debian and Ubuntu: Dash's apt repository

Dash publishes dashmate in a signed apt repository, `https://apt.services.dash.org/dashmate`, for amd64 and arm64. Its `stable` suite carries final releases.

Add the repository with these commands. They download the repository's signing key to a temporary file and install it only if its sha256 is `08006df56f24e282f19012a29e539dad431654abefefd697832ac61918b01230`, and they stop at the first command that fails. On a system without `sha256sum`, `shasum -a 256 -c -` does the same check.

```bash
(
set -e
keyring=$(mktemp)
trap 'rm -f "$keyring"' EXIT
curl --proto '=https' -fsSL https://apt.services.dash.org/dashmate/dashmate-archive-keyring.gpg -o "$keyring"
echo "08006df56f24e282f19012a29e539dad431654abefefd697832ac61918b01230  $keyring" | sha256sum -c -
sudo install -d -m 0755 /etc/apt/keyrings
sudo install -m 0644 "$keyring" /etc/apt/keyrings/dashmate-archive-keyring.gpg
sudo tee /etc/apt/sources.list.d/dashmate.sources > /dev/null <<'EOF'
Types: deb
URIs: https://apt.services.dash.org/dashmate
Suites: stable
Components: main
Architectures: amd64 arm64
Signed-By: /etc/apt/keyrings/dashmate-archive-keyring.gpg
EOF
sudo apt update
)
```

Then install dashmate:

```bash
sudo apt update && sudo apt install dashmate
```

To check the key by its fingerprint as well, run `gpg --show-keys /etc/apt/keyrings/dashmate-archive-keyring.gpg` (install `gnupg` if `gpg` is missing): the primary key is `A5F90926475EF25346DCE26836EF5BF853D100FE`, the fingerprint that [dashpay/platform#4246](https://github.com/dashpay/platform/issues/4246) gives as well.

The two files these commands write are, byte for byte, the ones the dashmate package ships, so dpkg takes them over without a question when it installs dashmate. The package adds the third file, the pin, itself (see [the files the package ships](#files-the-package-ships)). Later releases install with `sudo apt update && sudo apt install --only-upgrade dashmate` (see the [Update Guide](./update.md)).

### Debian package from the GitHub releases page

Instead of adding the repository first, you can download the dashmate package from the [GitHub releases page](https://github.com/dashpay/platform/releases/latest) and verify it with the release's `SHA256SUMS`. Installing it also adds Dash's apt repository, so later releases arrive through apt as above.

The block below downloads the newest final release: it reads `releases/latest`, which leaves prereleases out.
For a prerelease, use the [prerelease download](#downloading-a-prerelease-package) below, or the [`testing` suite](#prereleases).

The file name contains the version, so it changes with every release; this downloads the one matching the architecture you are on into a new private directory, so no other user can replace it before it is installed, and checks it against the `SHA256SUMS` file of the same release:

```bash
DASHMATE_DEB=""
DASHMATE_DEB_URL="$(curl -fsSL https://api.github.com/repos/dashpay/platform/releases/latest \
  | grep -o "https://[^\"]*_$(dpkg --print-architecture)\.deb" \
  | head -n 1)"
DASHMATE_DEB_DIR="$(mktemp -d)"
if [ -n "$DASHMATE_DEB_URL" ] && [ -n "$DASHMATE_DEB_DIR" ] && [ -d "$DASHMATE_DEB_DIR" ] \
  && curl -fL -o "$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")" "$DASHMATE_DEB_URL" \
  && curl -fL -o "$DASHMATE_DEB_DIR/SHA256SUMS" "$(dirname "$DASHMATE_DEB_URL")/SHA256SUMS" \
  && (cd "$DASHMATE_DEB_DIR" && awk -v f="$(basename "$DASHMATE_DEB_URL")" '$2 == f' SHA256SUMS | sha256sum -c -); then
  DASHMATE_DEB="$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")"
  echo "Downloaded $DASHMATE_DEB"
else
  echo "Could not download and verify the dashmate package. Try again later." >&2
fi
```

#### Downloading a prerelease package

To install a prerelease, open that release's page under [GitHub releases](https://github.com/dashpay/platform/releases) and copy the link to the `.deb` file for your architecture (`dpkg --print-architecture` prints it).
Download it with this block instead of the one above, replacing the example link with the one you copied:

```bash
DASHMATE_DEB=""
DASHMATE_DEB_URL="https://github.com/dashpay/platform/releases/download/v5.1.0-rc.2/dashmate_5.1.0.rc.2-1_amd64.deb"
DASHMATE_DEB_DIR="$(mktemp -d)"
if [ -n "$DASHMATE_DEB_DIR" ] && [ -d "$DASHMATE_DEB_DIR" ] \
  && curl -fL -o "$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")" "$DASHMATE_DEB_URL" \
  && curl -fL -o "$DASHMATE_DEB_DIR/SHA256SUMS" "$(dirname "$DASHMATE_DEB_URL")/SHA256SUMS" \
  && (cd "$DASHMATE_DEB_DIR" && awk -v f="$(basename "$DASHMATE_DEB_URL")" '$2 == f' SHA256SUMS | sha256sum -c -); then
  DASHMATE_DEB="$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")"
  echo "Downloaded $DASHMATE_DEB"
else
  echo "Could not download and verify the dashmate package. Check the link and try again." >&2
fi
```

Releases before 5.1.0-rc.1 have no `SHA256SUMS`, so the check fails for them.

#### Installing the downloaded package

Install the package that either download block saved. It is installed by its path, and nothing is installed if the download or the check failed:

```bash
if [ -n "$DASHMATE_DEB" ] && [ -f "$DASHMATE_DEB" ]; then
  sudo apt install "$DASHMATE_DEB"
else
  echo "No downloaded dashmate package found, so nothing was installed. Run the download step again in this shell." >&2
fi
```

> **Note:** At the end of the installation process, apt may display an error due to installing a downloaded package.
> You can ignore this error message:
> N: Download is performed unsandboxed as root as file '/tmp/tmp.k3Jd8sQw2L/dashmate_5.1.0-1_amd64.deb' couldn't be accessed by user '_apt'. - pkgAcquire::Run (13: Permission denied)

### Files the package ships

The dashmate package, from 5.1.0-rc.2 on, configures Dash's apt repository with three files:

- `/etc/apt/sources.list.d/dashmate.sources`, the repository source. It names the `stable` suite, which carries final releases only. To follow prereleases as well, add the [`testing` suite](#prereleases) in a file of its own.
- `/etc/apt/keyrings/dashmate-archive-keyring.gpg`, the key the repository is signed with. An unmodified copy is replaced on upgrade, which is how a renewed key arrives.
- `/etc/apt/preferences.d/dashmate.pref`, which limits the repository to the dashmate package: apt does not choose any other package from it as a candidate, in any architecture. This does not make it safe for the repository to list other packages. If it lists a package at the same version as the distribution with matching package metadata (the same control fields and size), apt can treat the two as one version even when their contents differ, and the pin then hides the distribution's copy too, including a security update. The repository's publisher accepts only the dashmate package, and that check is what keeps other packages out.

`apt remove dashmate` keeps the three files and `apt purge dashmate` deletes them. Once dashmate is removed, upgrades no longer renew the key, so when the key expires, `apt update` reports a signature error for the repository. To take dashmate off a host for good, purge it. To keep the source without using it, add `Enabled: no` to `dashmate.sources`; a later install keeps that line, so delete it to use the repository again.

Create these files only with the [commands above](#debian-and-ubuntu-dashs-apt-repository), which write them byte for byte as the package ships them, and do not change them. If one of them exists with different content when the package is installed or upgraded, dpkg asks whether to keep it. An install with no terminal to answer that question, such as an `apt-get -y` run from a script, fails at the prompt and leaves dashmate unconfigured. Anything else, such as the `testing` suite below, belongs in files of other names.

### Prereleases

Dash's apt repository also has a `testing` suite, which carries betas, release candidates and final releases, and a `dev` suite, which carries dev builds as well. To install prereleases, add `testing` in files of its own. These commands work the same way as the ones for `stable`; for dev builds, write `Suites: dev` instead of `Suites: testing`.

```bash
(
set -e
keyring=$(mktemp)
trap 'rm -f "$keyring"' EXIT
curl --proto '=https' -fsSL https://apt.services.dash.org/dashmate/dashmate-archive-keyring.gpg -o "$keyring"
echo "08006df56f24e282f19012a29e539dad431654abefefd697832ac61918b01230  $keyring" | sha256sum -c -
sudo install -d -m 0755 /etc/apt/keyrings
sudo install -m 0644 "$keyring" /etc/apt/keyrings/dashmate-testing-archive-keyring.gpg
sudo tee /etc/apt/sources.list.d/dashmate-testing.sources > /dev/null <<'EOF'
Types: deb
URIs: https://apt.services.dash.org/dashmate
Suites: testing
Components: main
Architectures: amd64 arm64
Signed-By: /etc/apt/keyrings/dashmate-testing-archive-keyring.gpg
EOF
sudo apt update
)
```

Then install or upgrade dashmate with `sudo apt install dashmate`. On a host without dashmate, the package then adds the `stable` source next to this one. Both suites have the same priority, so apt installs the highest version either one offers, and a final release sorts above its prereleases (`5.1.0-1` is newer than `5.1.0~rc.2-1`).

To stop following prereleases, delete the files again. dashmate stays at the version it has, since apt does not downgrade it, and the next final release on `stable` upgrades it:

```bash
sudo rm -f /etc/apt/sources.list.d/dashmate-testing.sources \
  /etc/apt/keyrings/dashmate-testing-archive-keyring.gpg \
  /etc/apt/preferences.d/dashmate-testing.pref
sudo apt update
```

`dashmate-testing.pref` is a pin that earlier versions of this guide had you create; the package's own `dashmate.pref` covers both suites. Those versions also suggested changing `Suites:` in `dashmate.sources` itself. That file then differs from the package's, so dpkg asks about it when a release changes it: set it back to `Suites: stable` and add `testing` as above instead.

dpkg does not own the `testing` files, so purging dashmate leaves them in place and the source active. To take dashmate off a host that uses them, delete them as well.

### Node package

> **Warning:** This installation option is not recommended for mainnet masternodes.
> Please install dashmate from Dash's apt repository or the GitHub releases page.

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
