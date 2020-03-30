# MN Bootstrap

> Distribution package for Dash Masternode installation

## Table of Contents

- [Pre-requisites](#Pre-requisites)
- [Install](#install)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Pre-requisites

* [Docker](https://docs.docker.com/engine/installation/)
* [Docker compose](https://docs.docker.com/compose/install/) (v1.25.0+)

## Install

Download and unzip [package](https://github.com/dashevo/mn-bootstrap/archive/master.zip).

## Usage

Package contains Docker Compose file and configuration presets.

### Configure

Package contains several configuration presets:
 - Local - standalone masternode for local development
 - Evonet - masternode with Evonet configuration
 - Testnet - masternode with testnet configuration

There are two ways to apply a present:
 1. Rename corresponding dotenv file (i.e. `.env.local`) to `.env`
 2. Add `--env-file` option to `docker-compose` command

### Start

In order to run a masternode use Docker Compose:

```bash
$ docker-compose up
```

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/mn-bootstrap/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
