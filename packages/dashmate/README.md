# MN Bootstrap

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/dashevo/mn-bootstrap)](https://github.com/dashevo/mn-bootstrap/releases)
[![Release Date](https://img.shields.io/github/release-date/dashevo/mn-bootstrap)](https://github.com/dashevo/mn-bootstrap/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg)](https://github.com/RichardLitt/standard-readme)

Distribution package for Dash Masternode installation

## Table of Contents

- [Install](#install)
- [Usage](#usage)
  - [Command line interface](#cli)
  - [Setup node](#setup-node)
  - [Configure node](#configure-node)
  - [Start node](#start-node)
  - [Stop node](#stop-node)
  - [Restart node](#restard-node)
  - [Register masternode](#register-masternode)
  - [Reset data](#reset-data)
  - [Full node](#full-node)
  - [Development](#development)
  - [Docker Compose](#docker-compose)
- [Contributing](#contributing)
- [License](#license)

## Install

### Dependencies

* [Docker](https://docs.docker.com/engine/installation/) (v18.06.0+)
* [Docker Compose](https://docs.docker.com/compose/install/) (v1.25.0+)
* [Node.js](https://nodejs.org/en/download/) (v10.0+)

For Linux installations you may optionally wish to follow the [post-installation steps](https://docs.docker.com/engine/install/linux-postinstall/) to manage Docker as a non-root user, otherwise you will have to run CLI and Docker commands with `sudo`.

### Distribution package

```bash
$ git clone -b master https://github.com/dashevo/mn-bootstrap.git
$ cd mn-bootstrap
$ npm install # optional: install CLI dependencies
$ sudo npm link # optional: link CLI for system-wide execution
```

## Usage

The package contains a CLI, Docker Compose and configuration files.

### CLI

The CLI can be used to perform routine tasks. Invoke the CLI with `mn` if linked during installation, or with `node bin/mn` if not linked. To list available commands, either run `mn` with no parameters or execute `mn help`. To list the help on any command just execute the command, followed by the `--help` option

### Setup node

The `setup` command is used to quickly configure common node configurations. Arguments may be provided as options, otherwise they will be queried interactively with sensible values suggested.

```
USAGE
  $ mn setup [PRESET] [NODE-TYPE]

ARGUMENTS
  PRESET     (testnet|evonet|local) Node configuration preset
  NODE-TYPE  (masternode|fullnode) Node type

OPTIONS
  -i, --external-ip=external-ip                            external ip
  -k, --operator-bls-private-key=operator-bls-private-key  operator bls private key
  -u, --update                                             download updated services before start
  -v, --verbose                                            use verbose mode for output
  --dapi-image-build-path=dapi-image-build-path            dapi's docker image build path
  --drive-image-build-path=drive-image-build-path          drive's docker image build path
```

To setup a testnet masternode:
```bash
$ mn setup testnet masternode
```

### Configure node

The `config` command is used to manage your node configuration before starting the node. Several system configurations are provided as a starting point:

 - base - basic config for use as template
 - local - standalone node for local development
 - evonet - node with Evonet configuration
 - testnet - node with testnet configuration

You can modify and use the system configs directly, or create your own. You can base your own configs on one of the system configs using the `mn config:create CONFIG [FROM]` command. You must set a default config with `mn config:default CONFIG` or specify a config with the `--config=<config>` option when running commands. The `base` config is initially set as default.

```
USAGE
  $ mn config

OPTIONS
  --config=config  configuration name to use

DESCRIPTION
  Display configuration options for default config

COMMANDS
  config:create   Create config
  config:default  Manage default config
  config:envs     Export config to envs
  config:get      Get config option
  config:list     List available configs
  config:remove   Remove config
  config:set      Set config option
```

### Start node

The `start` command is used to start a node with the default or specified config.

```
USAGE
  $ mn start
OPTIONS
  -u, --update                                     download updated services before start
  --config=config                                  configuration name to use
  --dapi-image-build-path=dapi-image-build-path    dapi's docker image build path
  --drive-image-build-path=drive-image-build-path  drive's docker image build path
```

To start a masternode:
```bash
$ mn start
```

### Stop node

The `stop` command is used to stop a running node.

```
USAGE
  $ mn stop
OPTIONS
  --config=config  configuration name to use
```

To stop a node:
```bash
$ mn stop
```

### Restart node

The `restart` command is used to restart a node with the default or specified config.

```
USAGE
  $ mn restart

OPTIONS
  -u, --update                                     download updated services before start
  --config=config                                  configuration name to use
  --dapi-image-build-path=dapi-image-build-path    dapi's docker image build path
  --drive-image-build-path=drive-image-build-path  drive's docker image build path
```

To update services and restart a masternode:
```bash
$ mn restart -u
```

### Register masternode

The `register` command creates a collateral funding transaction and then uses it to register a masternode on the specified network. It does not configure or start a masternode on the host.

#### Funding collateral

Before registering the masternode, you must have access to an address on the network you intend to use with a balance of more than 1000 Dash. 1000 Dash is used for the collateral transaction, and the remainder will be used for transaction fees. Make sure you have access to the private key for this address, since you will need to provide it in the next step. If using Dash Core, you can get the private key for a given address using the following command:

```
dumpprivkey "address"
```

If using a config specifying the `local` network, you can create and fund a new address using the `wallet` command as shown below.

```
USAGE
  $ mn wallet:mint AMOUNT
ARGUMENTS
  AMOUNT  amount of dash to be generated to address
OPTIONS
  -a, --address=address  recipient address instead of a new one
  --config=config        configuration name to use
```

To generate 1001 Dash to a new address:
```bash
mn wallet:mint 1001
```

#### Masternode registration

Run the `register` command as described below. The command will first verify sufficient balance on the funding address from the previous step. It will then generate new addresses for the collateral, owner and operator and display the addresses and associated private keys as output. The collateral of exactly 1000 Dash will be sent from the funding address to the collateral address, and after 15 blocks have been mined, the registration transaction will be broadcast on the network. Assuming a properly configured and running masternode exists at the specified IP address and port, it should become active after the registration transaction has been mined to a block on the network.

```
USAGE
  $ mn register FUNDING-PRIVATE-KEY
ARGUMENTS
  FUNDING-PRIVATE-KEY  private key with more than 1000 dash for funding collateral
OPTIONS
  --config=config  configuration name to use
```

To register a masternode:
```bash
$ mn register cVdEfkXLHqftgXzRYZW4EdwtcnJ8Mktw9L4vcEcqbVDs3e2qdzCf
```

### Reset data

The `reset` command removes all data corresponding to the specified config and allows you to start a node from scratch.

```
USAGE
  $ mn reset
OPTIONS
  -h, --hard           reset config as well as data
  -p, --platform-only  reset platform data only
  --config=config      configuration name to use
```

To reset a node:
```bash
$ mn reset
```

### Show status

The `status` command outputs status information relating to either the host, masternode or services.

```
USAGE
  $ mn status:COMMAND

COMMANDS
  status:host        Show host status details
  status:masternode  Show masternode status details
  status:services    Show service status details
```

To show the host status:
```bash
$ mn status:host
```

### Full node

It is also possible to start a full node instead of a masternode. Modify the config setting as follows:

```bash
mn config:set core.masternode.enable false
```

### Development

When developing on a standalone node (a config specifying the `local` network), `setup local` can be used to generate some dash, register a masternode and populate the node with the data required for local development.

To allow developers quickly test changes to DAPI and Drive, a local path for DAPI or Drive may be specified via the `--drive-image-build-path` and `--dapi-image-build-path` options of the `start` command. A Docker image will be built from the provided path and then used by mn-bootstrap.

### Docker Compose

If you want to use Docker Compose directly, you will need to pass a configuration as a dotenv file. You can output a config to a dotenv file for Docker Compose as follows:

```bash
$ mn config:envs --output-file .env
```

Docker Compose will attempt to read a file named `.env` by default. You can optionally specify a dotenv file with a different filename for Docker Compose by adding `--env-file` option to the `docker-compose` command as follows:

```bash
$ docker-compose --env-file=<filename>
```

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/mn-bootstrap/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
