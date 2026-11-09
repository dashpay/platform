# Updating Dashmate

This guide explains how to update Dashmate and your node to the latest version.

## Basic Update

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

### Example update process:

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
║ rs-dapi          │ dashpay/rs-dapi:0.24         │ updated    ║
║ Gateway          │ dashpay/envoy:0.24           │ updated    ║
║ Dashmate Helper  │ dashpay/dashmate-helper:0.24 │ updated    ║
╚══════════════════╧══════════════════════════════╧════════════╝
$ dashmate start
```

You can also use JSON format for programmatic access:

```bash
$  dashmate update --format=json --config local_1 | jq
```

```json
[
  {
    "name": "dashmate_helper",
    "title": "Dashmate Helper",
    "updated": "error",
    "image": "dashpay/dashmate-helper:3.0.0-dev.1"
  },
  {
    "name": "core",
    "title": "Core",
    "updated": "up to date",
    "image": "dashpay/dashd:23"
  },
  {
    "name": "drive_abci",
    "title": "Drive ABCI",
    "updated": "error",
    "image": "drive:local"
  },
  {
    "name": "drive_tenderdash",
    "title": "Drive Tenderdash",
    "updated": "up to date",
    "image": "dashpay/tenderdash:1.5"
  },
  {
    "name": "rs_dapi",
    "title": "rs-dapi (Rust DAPI)",
    "updated": "error",
    "image": "rs-dapi:local"
  },
  {
    "name": "gateway",
    "title": "Gateway",
    "updated": "up to date",
    "image": "dashpay/envoy:1.39.0-impr.1"
  }
]
```

## Dash Core v24

Dashmate 5.0.4 moves mainnet and testnet nodes on the stock `dashpay/dashd:23` image to `dashpay/dashd:24`; `dashmate update` pulls it and `dashmate restart --safe` starts it. A node with an image you set yourself keeps it.

Local networks and devnets keep their nodes on Core v23, since a Core v24 node discards the sporks a v23 node stored and does not request them from v23 peers. Move such a network as a whole: set `core.docker.image` on every node, restart them, then sign the sporks again on the node holding the spork key (`local_seed` on a local network), e.g. `dashmate core cli "sporkupdate SPORK_17_QUORUM_DKG_ENABLED 0" --config=local_seed`. For a local network, `dashmate group reset --hard` followed by `dashmate setup local` gives a fresh network on Core v24.

## Major Updates with Platform Reset

In some cases, you must also additionally reset platform data:

* When the upgrade contains non-compatible changes (e.g. switching between v22/v23)
* When the `dashmate setup` command exited with errors or was interrupted
* When the platform layer was wiped on the network

The reset and update process for major updates:

```bash
$ dashmate stop
$ npm install -g dashmate
$ dashmate reset --platform --hard
$ dashmate update
$ dashmate setup
$ dashmate start
```

## Local Network Updates

Before applying an upgrade to a local network, the network should be stopped and reset with `dashmate reset --hard`.

## Notes on Compatibility

When upgrading between major versions, always check the release notes for any breaking changes or special upgrade instructions.
