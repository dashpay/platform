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
This installs stable releases only: the snippet below reads `releases/latest`, which leaves prereleases out.
If you run a prerelease, download the package for your architecture from that release's page instead.

The file name contains the version, so it changes with every release; this downloads the one matching the architecture you are on into a new private directory, so no other user can replace it before it is installed:

```bash
DASHMATE_DEB_URL="$(curl -fsSL https://api.github.com/repos/dashpay/platform/releases/latest \
  | grep -o "https://[^\"]*_$(dpkg --print-architecture)\.deb" \
  | head -n 1)"
DASHMATE_DEB="$(mktemp -d)/$(basename "$DASHMATE_DEB_URL")"
if [ -n "$DASHMATE_DEB_URL" ] && curl -fL -o "$DASHMATE_DEB" "$DASHMATE_DEB_URL"; then
  echo "Downloaded $DASHMATE_DEB"
else
  DASHMATE_DEB=""
  echo "Could not download the dashmate package. Try again later." >&2
fi
```

Install the package that was just downloaded. Nothing is installed if the download failed:

```bash
if [ -n "$DASHMATE_DEB" ] && [ -f "$DASHMATE_DEB" ]; then
  sudo apt update
  sudo apt install "$DASHMATE_DEB"
else
  echo "No downloaded dashmate package found, so nothing was installed. Run the download step again in this shell." >&2
fi
```

> **Note:** At the end of the installation process, apt may display an error due to installing a downloaded package.
> You can ignore this error message:
> N: Download is performed unsandboxed as root as file '/tmp/tmp.k3Jd8sQw2L/dashmate_5.1.0-1_amd64.deb' couldn't be accessed by user '_apt'. - pkgAcquire::Run (13: Permission denied)

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
