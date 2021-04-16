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
  - [Restart node](#restart-node)
  - [Show node status](#show-node-status)
  - [Reset node data](#reset-node-data)
  - [Full node](#full-node)
  - [Node groups](#node-groups)
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

The CLI can be used to perform routine tasks. Invoke the CLI with `mn` if linked during installation, or with `node bin/mn` if not linked. To list available commands, either run `mn` with no parameters or execute `mn help`. To list the help on any command just execute the command, followed by the `--help` option.

### Setup node

The `setup` command is used to quickly configure common node configurations. Arguments may be provided as options, otherwise they will be queried interactively with sensible values suggested.

```
USAGE
  $ mn setup [PRESET] [NODE-TYPE]

ARGUMENTS
  PRESET     (testnet|local) Node configuration preset
  NODE-TYPE  (masternode|fullnode) Node type

OPTIONS
  -i, --external-ip=external-ip                            external ip
  -k, --operator-bls-private-key=operator-bls-private-key  operator bls private key
  -p, --funding-private-key=funding-private-key            private key with more than 1000 dash for funding collateral
  -v, --verbose                                            use verbose mode for output
  --node-count=node-count                                  number of nodes to setup
```

Supported presets:
 * `testnet` - a masternode or full node for testnet
 * `local` - a node group to run a local dash network with the specified number of masternodes. To operate a group of nodes, use the [group commands](#node-groups)

To setup a testnet masternode:
```bash
$ mn setup testnet masternode
```

#### Masternode registration

If a funding private key is provided with the `--funding-private-key` option, the tool will automatically register your node on the network as a masternode. This functionality is only available when using the `testnet` preset.

### Configure node

The `config` command is used to manage your node configuration before starting the node. Several system configurations are provided as a starting point:

 - base - basic config for use as template
 - local - template for local node configs
 - testnet - testnet node configuration

You can modify and use the system configs directly, or create your own. You can base your own configs on one of the system configs using the `mn config:create CONFIG [FROM]` command. You must set a default config with `mn config:default CONFIG` or specify a config with the `--config=<config>` option when running commands. The `base` config is initially set as default.

```
USAGE
  $ mn config

OPTIONS
  -v, --verbose    use verbose mode for output
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
  -v, --verbose             use verbose mode for output
  -w, --wait-for-readiness  wait for nodes to be ready
  --config=config           configuration name to use
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
  -v, --verbose    use verbose mode for output
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
  -v, --verbose    use verbose mode for output
  --config=config  configuration name to use
```

### Show node status

The `status` command outputs status information relating to either the host, masternode or services.

```
USAGE
  $ mn status

OPTIONS
  -v, --verbose    use verbose mode for output
  --config=config  configuration name to use

COMMANDS
  status:core        Show core status details
  status:host        Show host status details
  status:masternode  Show masternode status details
  status:platform    Show platform status details
  status:services    Show service status details
```

To show the host status:
```bash
$ mn status:host
```

### Reset node data

The `reset` command removes all data corresponding to the specified config and allows you to start a node from scratch.

```
USAGE
  $ mn reset

OPTIONS
  -v, --verbose        use verbose mode for output
  --config=config      configuration name to use
  -h, --hard           reset config as well as data
  -p, --platform-only  reset platform data only
```

With the hard reset mode enabled, the corresponding config will be reset as well. To proceed, running the node [setup](#setup-node) is required.

To reset a node:
```bash
$ mn reset
```

### Full node

It is also possible to start a full node instead of a masternode. Modify the config setting as follows:

```bash
mn config:set core.masternode.enable false
```

### Node groups

CLI allows to [setup](#setup-node) and operate multiple nodes. Only the `local` preset is supported at the moment.

#### Default group

The [setup](#setup-node) command set corresponding group as default. To output the current default group or set another one as default use `group:default` command.

```
USAGE
  $ mn group:default [GROUP]

ARGUMENTS
  GROUP  group name

OPTIONS
  -v, --verbose  use verbose mode for output
```

#### List group configs

The `group:list` command outputs a list of group configs.

```
USAGE
  $ mn group:list

OPTIONS
  -v, --verbose  use verbose mode for output
  --group=group  group name to use
```

#### Start group nodes

The `group:start` command is used to start a group of nodes belonging to the default group or a specified group.

```
USAGE
  $ mn group:start

OPTIONS
  -v, --verbose             use verbose mode for output
  -w, --wait-for-readiness  wait for nodes to be ready
  --group=group             group name to use
```

#### Stop group nodes

The `group:stop` command is used to stop group nodes belonging to the default group or a specified group.

```
USAGE
  $ mn group:stop

OPTIONS
  -v, --verbose  use verbose mode for output
  --group=group  group name to use
```

#### Restart group nodes

The `group:restart` command is used to restart group nodes belonging to the default group or a specified group.

```
USAGE
  $ mn group:restart

OPTIONS
  -v, --verbose  use verbose mode for output
  --group=group  group name to use
```

#### Show group status

The `group:status` command outputs group status information.

```
USAGE
  $ mn group:status

OPTIONS
  -v, --verbose  use verbose mode for output
  --group=group  group name to use
```

#### Reset group nodes

The `group:reset` command removes all data corresponding to the specified group and allows you to start group nodes from scratch.

```
USAGE
  $ mn group:reset

OPTIONS
  -h, --hard           reset config as well as data
  -p, --platform-only  reset platform data only
  -v, --verbose        use verbose mode for output
  --group=group        group name to use
```

With the hard reset mode enabled, corresponding configs will be reset as well. To proceed, running the node [setup](#setup-node) is required.

#### Create config group

To group nodes together, set a group name to `group` option in corresponding configs.

Create a group of two testnet nodes:
```bash
# create a new config using `testnet` config as template
mn config:create testnet_2 testnet

# combine configs into the group
mn config:set --config=testnet group testnet
mn config:set --config=testnet_2 group testnet

# set the group as default
mn group:default testnet
```

To start the group of nodes, ports and other required options need to be updated.

### Development

To start a local dash network, the `setup` command with the `local` preset can be used to generate configs, mine some dash, register masternodes and populate the nodes with the data required for local development.

To allow developers quickly test changes to DAPI and Drive, a local path for DAPI or Drive may be specified via the `platform.drive.abci.docker.build.path` and `platform.dapi.api.docker.build.path` config options. A Docker image will be built from the provided path and then used by mn-bootstrap.

### Docker Compose

If you want to use Docker Compose directly, you will need to pass a configuration as a dotenv file. You can output a config to a dotenv file for Docker Compose as follows:

```bash
$ mn config:envs --config=testnet --output-file .env.testnet
```

Then specify the created dotenv file as an option for the `docker-compose` command:

```bash
$ docker-compose --env-file=.env.testnet
```

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/mn-bootstrap/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
