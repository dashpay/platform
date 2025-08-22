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

Download the newest dashmate installation package for your architecture from the [GitHub releases page](https://github.com/dashpay/platform/releases/latest):

```bash
wget https://github.com/dashpay/platform/releases/download/v1.8.0/dashmate_1.8.0.e4e156c86-1_amd64.deb
```

Install dashmate using apt:

```bash
sudo apt update
sudo apt install ./dashmate_1.8.0.e4e156c86-1_amd64.deb
```

> **Note:** At the end of the installation process, apt may display an error due to installing a downloaded package.
> You can ignore this error message:
> N: Download is performed unsandboxed as root as file '/home/ubuntu/dashmate_1.8.0.e4e156c86-1_amd64.deb' couldn't be accessed by user '_apt'. - pkgAcquire::Run (13: Permission denied)

### Node package

> **Warning:** This installation option is not recommended for mainnet masternodes.
> Please install packages from the GitHub releases page.

To install the NPM package, it is necessary to install Node.JS first. We recommend installing it using [nvm](https://github.com/nvm-sh/nvm#readme):

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.5/install.sh | bash
source ~/.bashrc
nvm install 20
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
