# Dashmate

[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashpay/platform)](https://github.com/dashpay/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg)](https://github.com/RichardLitt/standard-readme)

Distribution package for Dash node installation

## Table of Contents

- [Install](#install)
- [Update](#update)
- [Usage](#usage)
  - [Command line interface](#cli)
  - [Node setup](#node-setup)
  - [Configure node](#configure-node)
  - [Start node](#start-node)
  - [Stop node](#stop-node)
  - [Restart node](#restart-node)
  - [Show node status](#show-node-status)
  - [Execute Core CLI command](#execute-core-cli-command)
  - [Reset node data](#reset-node-data)
  - [Full node](#full-node)
  - [Node groups](#node-groups)
  - [Development](#development)
  - [Docker Compose](#docker-compose)
- [Contributing](#contributing)
- [License](#license)

## Install

### Dependencies

* [Docker](https://docs.docker.com/engine/installation/) (v20.10+)
* [Node.js](https://nodejs.org/en/download/) (v20, NPM v8.0+)

For Linux installations you may optionally wish to follow the Docker [post-installation steps](https://docs.docker.com/engine/install/linux-postinstall/) to manage Docker as a non-root user, otherwise you will have to run CLI and Docker commands with `sudo`.

### Distribution package

Use NPM to install dashmate globally in your system:
```bash
$ npm install -g dashmate
```

## Update

The `update` command is used to quickly get the latest patches for dashmate components. It is necessary to restart the node after the update is complete.

```
USAGE
  $ dashmate update [-v] [--config <value>]

FLAGS
  -v, --verbose     use verbose mode for output
  --config=<value>  configuration name to use
  --format=<option>  [default: plain] display output format
                   <options: json|plain>
```

Example usage:

```bash
$ dashmate stop
$ npm install -g dashmate
$ dashmate update
╔══════════════════╤══════════════════════════════╤════════════╗
║ Service          │ Image                        │ Updated    ║
╟──────────────────┼──────────────────────────────┼────────────╢
║ Core             │ dashpay/dashd:19.1.0         │ up to date ║
║ Drive ABCI       │ dashpay/drive:0.24           │ updated    ║
║ Drive Tenderdash │ dashpay/tenderdash:0.11.2    │ up to date ║
║ DAPI API         │ dashpay/dapi:0.24            │ updated    ║
║ Gateway          │ dashpay/envoy:0.24           │ updated    ║
║ Dashmate Helper  │ dashpay/dashmate-helper:0.24 │ updated    ║
╚══════════════════╧══════════════════════════════╧════════════╝
$ dashmate update --format=json 
[{"name":"core","title":"Core","updated":false,"image":"dashpay/dashd:19.2.0"},{"name":"drive_abci","title":"Drive ABCI","pulled":false,"image":"dashpay/drive:0.24"},{"name":"drive_tenderdash","title":"Drive Tenderdash","pulled":true,"image":"dashpay/tenderdash:0.11.2"},{"name":"dapi_api","title":"DAPI API","pulled":false,"image":"dashpay/dapi:0.24"},{"name":"gateway","title":"Gateway","pulled":false,"image":"dashpay/envoy:0.24"},{"name":"dashmate_helper","title":"Dashmate Helper","pulled":false,"image":"dashpay/dashmate-helper:0.24"}]
$ dashmate start
```

In some cases, you must also additionally reset platform data:

* Upgrade contains non-compatible changes (e.g. switching between v22/v23)
* The ``dashmate setup`` command exited with errors or was interrupted
* The platform layer was wiped on the network

```bash
$ dashmate stop
$ npm install -g dashmate
$ dashmate reset --platform-only --hard
$ dashmate update
$ dashmate setup
$ dashmate start
```

Before applying an upgrade, the local network should be stopped and reset with ``dashmate reset --hard``. 

## Usage

The package contains a CLI, Docker Compose and configuration files.

### CLI

The CLI can be used to perform routine tasks. Invoke the CLI with `dashmate`. To list available commands, either run `dashmate` with no parameters or execute `dashmate help`. To list the help on any command, execute the command followed by the `--help` option.

### Node setup

The `setup` command is used to quickly configure common node configurations. Arguments may be provided as options, otherwise they will be queried interactively with sensible values suggested.

```
USAGE
  $ dashmate setup [PRESET] [-v] [-d] [-c <value>] [-m <value>]

ARGUMENTS
  PRESET  (mainnet|testnet|local) Node configuration preset

FLAGS
  -c, --node-count=<value>      number of nodes to set up
  -d, --[no-]debug-logs         enable debug logs
  -m, --miner-interval=<value>  interval between blocks
  -v, --verbose                 use verbose mode for output

DESCRIPTION
  Set up a new Dash node
```

Supported presets:
 * `mainnet` - a node connected to the Dash main network
 * `testnet` - a node connected to the Dash test network
 * `local` - a full network environment on your machine for local development. To operate a group of nodes, use the [group commands](#node-groups)

To set up a testnet node:
```bash
$ dashmate setup testnet
```

### Configure node

The `config` command is used to manage your node configuration before starting the node. Several system configurations are provided as a starting point:

 - base - basic config for use as template
 - local - template for local node configs
 - testnet - testnet node configuration
 - mainnet - mainnet node configuration

You can modify and use the system configs directly, or create your own. You can base your own configs on one of the system configs using the `dashmate config create CONFIG [FROM]` command. You must set a default config with `dashmate config default CONFIG` or specify a config with the `--config=<config>` option when running commands. The `base` config is initially set as default.

```
USAGE
  $ dashmate config [-v] [--config <value>]

FLAGS
  -v, --verbose     use verbose mode for output
  --config=<value>  configuration name to use

DESCRIPTION
  Show default config

COMMANDS
  config create   Create new config
  config default  Manage default config
  config envs     Export config to envs
  config get      Get config option
  config list     List available configs
  config remove   Remove config
  config render   Render config's service configs
  config set      Set config option
```

### Start node

The `start` command is used to start a node with the default or specified config.

```
USAGE
  $ dashmate start [-v] [--config <value>] [-w]

FLAGS
  -f, --force               force start even if any services are already running
  -p, --platform            start only platform
  -v, --verbose             use verbose mode for output
  -w, --wait-for-readiness  wait for nodes to be ready
  --config=<value>          configuration name to use
```

To start a masternode:
```bash
$ dashmate start
```

### Stop node

The `stop` command is used to stop a running node.

```
USAGE
  $ dashmate stop [--config <value>] [-v] [-f] [-p] [-s]

FLAGS
  -f, --force       force stop even if any service is running
  -p, --platform    stop only platform
  -s, --safe        wait for dkg before stop
  -v, --verbose     use verbose mode for output
  --config=<value>  configuration name to use

```

To stop a node:
```bash
$ dashmate stop
```

### Restart node

The `restart` command is used to restart a node with the default or specified config.

```
USAGE
  $ dashmate restart [--config <value>] [-v] [-p] [-s]

FLAGS
  -p, --platform        restart only platform
  -s, --safe            wait for dkg before stop
  -v, --verbose         use verbose mode for output
      --config=<value>  configuration name to use
```

### Show node status

The `status` command outputs status information relating to either the host, masternode or services.

```
USAGE
  $ dashmate status [-v] [--config <value>] [--format json|plain]

FLAGS
  -v, --verbose      use verbose mode for output
  --config=<value>   configuration name to use
  --format=<option>  [default: plain] display output format
                     <options: json|plain>

COMMANDS
  status core        Show core status details
  status host        Show host status details
  status masternode  Show masternode status details
  status platform    Show platform status details
  status services    Show service status details
```

To show the host status:
```bash
$ dashmate status host
```

### Execute Core CLI command

The `core cli` command executes an `dash-cli` command to the core container on the current config.

```
USAGE
  $ dashmate core cli [COMMAND] [--config <value>]

ARGUMENTS
  COMMAND dash-cli command written in the double quotes 

FLAGS
  --config=<value>  configuration name to use

DESCRIPTION
  Dash Core CLI
```

Example:
```bash
$ dashmate core cli "getblockcount"
1337
```

### Reset node data

The `reset` command removes all data corresponding to the specified config and allows you to start a node from scratch.

```
USAGE
  $ dashmate reset [-v] [--config <value>] [-h] [-f] [-p]

FLAGS
  -f, --force          skip running services check
  -h, --hard           reset config as well as data
  -p, --platform-only  reset platform data only
  -v, --verbose        use verbose mode for output
  --config=<value>     configuration name to use
```

To reset a node:
```bash
$ dashmate reset
```

#### Hard reset
With the hard reset mode enabled, the corresponding config will be reset in addition to the platform data. After a hard reset, it is necessary to run the node [setup](#node-setup) to proceed.
```bash
$ dashmate reset --hard
```

#### Manual reset
Manual reset can be used if the local setup is corrupted and a hard reset does not fix it. This could happen due to dashmate configuration incompatibilities after a major upgrade, leaving you unable to execute any commands.
```bash
docker stop $(docker ps -q)
docker system prune
docker volume prune
rm -rf ~/.dashmate/
```


### Reindex core chain data

The `core reindex` command helps you to reindex your Core instance in the node.

The process displays interactive progress and may be interrupted at any time. After reindex is finished core and other services will become online without any interactions from the user.

The `core reindex` command works for regular and local configurations.

```
USAGE
  $ dashmate core reindex [-v] [--config <value>] [-d] [-f]

FLAGS
  -d, --detach      run the reindex process in the background
  -f, --force       reindex already running node without confirmation
  -v, --verbose     use verbose mode for output
  --config=<value>  configuration name to use

DESCRIPTION
  Reindex Core data
```

### Full node
It is also possible to start a full node instead of a masternode. Modify the config setting as follows:
```bash
dashmate config set core.masternode.enable false
```


### Node groups

The CLI allows [setup](#node-setup) and operation of multiple nodes. Only the `local` preset is supported at the moment.

#### Default group

The [setup](#node-setup) command sets the corresponding group as default. To output the current default group or set another one as default, use the `group:default` command.

```
USAGE
  $ dashmate group default [GROUP] [-v]

ARGUMENTS
  GROUP  group name

FLAGS
  -v, --verbose  use verbose mode for output
```

#### List group configs

The `group list` command outputs a list of group configs.

```
USAGE
  $ dashmate group list [-v] [--group <value>]

FLAGS
  -v, --verbose    use verbose mode for output
  --group=<value>  group name to use
```

#### Start group nodes

The `group start` command is used to start a group of nodes belonging to the default group or a specified group.

```
USAGE
  $ dashmate group start [-v] [--group <value>] [-w]

FLAGS
  -v, --verbose             use verbose mode for output
  -w, --wait-for-readiness  wait for nodes to be ready
  --group=<value>           group name to use
```

#### Stop group nodes

The `group stop` command is used to stop group nodes belonging to the default group or a specified group.

```
USAGE
  $ dashmate group stop [-v] [--group <value>] [-f]

FLAGS
  -f, --force      force stop even if any is running
  -s, --safe       wait for dkg before stop
  -v, --verbose    use verbose mode for output
  --group=<value>  group name to use
```

#### Restart group nodes

The `group restart` command is used to restart group nodes belonging to the default group or a specified group.

```
USAGE
  $ dashmate group restart [--group <value>] [-v] [-s]

FLAGS
  -s, --safe           wait for dkg before stop
  -v, --verbose        use verbose mode for output
      --group=<value>  group name to use

DESCRIPTION
  Restart group nodes
```

#### Show group status

The `group status` command outputs group status information.

```
USAGE
  $ dashmate group status [-v] [--group <value>] [--format json|plain]

FLAGS
  -v, --verbose      use verbose mode for output
  --format=<option>  [default: plain] display output format
                     <options: json|plain>
  --group=<value>    group name to use
```

#### Reset group nodes

The `group reset` command removes all data corresponding to the specified group and allows you to start group nodes from scratch.

```
USAGE
  $ dashmate group reset [-v] [--group <value>] [--hard] [-f] [-p]

FLAGS
  -f, --force          reset even running node
  -p, --platform-only  reset platform data only
  -v, --verbose        use verbose mode for output
  --group=<value>      group name to use
  --hard               reset config as well as data
```

With hard reset mode enabled, the corresponding node configs will be reset as well. It will be necessary to run node [setup](#node-setup) again from scratch to start a new local node group.

#### Reindex group nodes

The `group core reindex` reindexes all your local dash core containers

```
USAGE
  $ dashmate group core reindex [-v] [--group <value>] [-d] [-f]

FLAGS
  -d, --detach     run the reindex process in the background
  -f, --force      reindex already running node without confirmation
  -v, --verbose    use verbose mode for output
  --group=<value>  group name to use

DESCRIPTION
  Reindex group Core data
```

With hard reset mode enabled, the corresponding node configs will be reset as well. It will be necessary to run node [setup](#node-setup) again from scratch to start a new local node group.

#### Mint tDash

The `wallet mint` command can be used to generate an arbitrary amount of tDash to a new or specified recipient address on a local network. The network must be stopped before running this command.

```
USAGE
  $ dashmate wallet mint [AMOUNT] [-v] [--config <value>] [-a <value>]

ARGUMENTS
  AMOUNT  amount of tDash to be generated to address

FLAGS
  -a, --address=<value>  use recipient address instead of creating new
  -v, --verbose          use verbose mode for output
  --config=<value>       configuration name to use
```

#### Create config group

To group nodes together, set a group name using the `group` option with the corresponding configs.

Create a group of two testnet nodes:
```bash
# create a new config using `testnet` config as template
dashmate config create testnet_2 testnet

# combine configs into the group
dashmate config set --config=testnet group testnet
dashmate config set --config=testnet_2 group testnet

# set the group as default
dashmate group default testnet
```

#### Render config's service configs

If you changed your config manually and you'd like to dashmate to render
again all your service configs (dashd.conf, config.toml, etc.), you can issue that command.

```bash
dashmate config render
"testnet" service configs rendered
```

### Development

To start a local dash network, the `setup` command with the `local` preset can be used to generate configs, mine some tDash, register masternodes, and populate the nodes with the data required for local development.

To allow developers to quickly test changes to DAPI and Drive, a local path for this repository may be specified using the `platform.sourcePath` config options. A Docker image will be built from the provided path and then used by Dashmate.

### Docker Compose

If you want to use Docker Compose directly, you will need to pass in configuration as a dotenv file. You can output a config to a dotenv file for Docker Compose as follows:

```bash
$ dashmate config envs --config=testnet --output-file .env.testnet
```

Then specify the created dotenv file as an option for the `docker compose` command:

```bash
$ docker compose --env-file=.env.testnet up -d
```

## Troubleshooting

#### [FAILED] Node is not running
One of your nodes is not running, you may retry with the --force option:

`dashmate stop --force` to stop single node (fullnode / masternode)

`dashmate group:stop --force` to stop group of nodes (local)

#### Running services detected. Please ensure all services are stopped for this config before starting
Some nodes are still running and preventing dashmate from starting properly. This may occur after a command exits with an error. Try to force stop the nodes using the `--force` option before trying to run the `start` command again.

`dashmate stop --force` to stop single node (fullnode / masternode)

`dashmate group:stop --force` to stop group of nodes (local)

#### externalIp option is not set in base config
This may happen when you switch back and forth between major versions, making config incompatible. In this case, do a manual reset and run setup again.

#### TypeError Plugin: dashmate: Cannot read properties of undefined (reading 'dash')
This can occur if other `.yarnrc` and `node_modules` directories exist in parent directories. Check your home directory for any `.yarnrc` and `node_modules`, delete them all and try again.


## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
