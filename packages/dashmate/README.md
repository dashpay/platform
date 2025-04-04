# Dashmate

[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashpay/platform)](https://github.com/dashpay/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg)](https://github.com/RichardLitt/standard-readme)

Distribution package for Dash node installation

## Table of Contents

- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Documentation

- [Installation Guide](./docs/installation.md)
- [Update Guide](./docs/update.md)
- [Usage Guide](./docs/usage.md)
- [Services Documentation](./docs/services/index.md)
- [Configuration Options](./docs/config/index.md)
- [Command Reference](./docs/commands/index.md)
- [Troubleshooting Guide](./docs/troubleshooting.md)

## Quick Start

### Install

```bash
$ npm install -g dashmate
```

For detailed installation instructions, see the [Installation Guide](./docs/installation.md).

### Basic Usage

```bash
# Set up a testnet node
$ dashmate setup testnet

# Start the node
$ dashmate start

# Check node status
$ dashmate status

# Stop the node
$ dashmate stop
```

For detailed usage instructions, see the [Usage Guide](./docs/usage.md).

## Troubleshooting

For common issues and solutions, see the [Troubleshooting Guide](./docs/troubleshooting.md).


## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
