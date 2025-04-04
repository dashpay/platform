# Installation

This guide provides instructions for installing Dashmate, a distribution package for Dash node installation.

## Dependencies

Before installing Dashmate, ensure you have the following dependencies installed:

* [Docker](https://docs.docker.com/engine/installation/) (v20.10+)
* [Node.js](https://nodejs.org/en/download/) (v20, NPM v8.0+)

For Linux installations, you may optionally wish to follow the Docker [post-installation steps](https://docs.docker.com/engine/install/linux-postinstall/) to manage Docker as a non-root user. Otherwise, you will have to run CLI and Docker commands with `sudo`.

## Installing Dashmate

Use NPM to install dashmate globally in your system:

```bash
$ npm install -g dashmate
```

This will make the `dashmate` command available system-wide.

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