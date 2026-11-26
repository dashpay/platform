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

## Major Updates with Platform Reset

### The network restarted Platform from a new genesis

On testnet and devnets the network may restart Platform from a new genesis. Reset only the Platform data, with the soft reset. It keeps the node's Platform identity, endpoints and certificate; the new genesis arrives with the dashmate release as a config migration. No setup is needed:

```bash
$ dashmate stop
$ npm install -g dashmate
$ dashmate reset --platform
$ dashmate update
$ dashmate start
```

### Starting Platform over

`dashmate reset --platform --hard` also sets every Platform option back to the preset's defaults: the node key, the Platform ports, the certificate provider and the certificate. Nothing of Platform survives it. On mainnet it also disables Platform, because the mainnet preset does. Set Platform up again with `dashmate setup --platform`, which configures only Platform and leaves Core as it is.

An evonode's node key is bound to its registration (the ProRegTx or a later ProUpServTx records its node ID), and the hard reset keeps no copy of it, so the flow starts with a backup into a file only you can read. The parentheses keep the `umask` from staying set in your shell:

```bash
$ dashmate stop --platform --config testnet            # skip if Platform is not running
$ (umask 077; dashmate config get platform.drive.tenderdash.node.key --config testnet > node-key)
$ dashmate reset --platform --hard --config testnet
$ dashmate setup --platform testnet --platform-node-key-file node-key
$ dashmate update --config testnet                     # now pulls the Platform images
$ dashmate start --platform --config testnet           # or `dashmate start --config testnet` if Core is stopped
```

`setup --platform` reads the registration from the masternode list, through Core, and accepts only the key whose node ID is registered. If Core is stopped it starts Core for the duration of the command and waits until Core has synced. It also obtains the gateway certificate.

Run setup before `update`. On mainnet the reset disables Platform, so an earlier `update` pulls no Platform images, and `dashmate start` runs Core alone without saying so: nothing reminds you that Platform still has to be set up. On testnet an earlier `update` says that Platform must be set up first and skips its certificate check.

If `platform.quorumList.enabled` is `true`, stop the whole node with `dashmate stop --config testnet` instead: `stop --platform` also stops Core then.

To combine this with a dashmate upgrade, do the normal upgrade first (stop the node, install the new version, check `dashmate --version`), then continue from the backup above; Platform is already stopped.

Unattended, with the key read from stdin rather than the command line:

```bash
$ dashmate setup --platform testnet --non-interactive --ssl-provider letsencrypt \
    --platform-node-key-file - < node-key
```

See [setup](./commands/setup.md#platform-only-setup) for every flag.

### Lost node key

dashmate cannot recover a deleted node key, and it does not change a registration. Register a new key first, then set Platform up with it:

1. Generate a key in DMT, or with `tenderdash gen-node-key` where Tenderdash is installed: it prints a `node_key.json`; redirect it into a file only you can read.
2. Submit a ProUpServTx with the new node ID and the existing endpoints: DMT "Update service", or `protx update_service_evo` in the Dash-Qt console. Its third argument is the operator BLS private key, so do not paste it into a shell. Update the DMT entry too, or a later DMT "Update service" sends the old node ID again.
3. Wait until the transaction is mined, then run `dashmate setup --platform --config testnet --platform-node-key-file <file>`. The check confirms the new registration.

Until then dashmate refuses to start Platform on the node. To run Core alone in the meantime, disable Platform with `dashmate config set platform.enable false --config testnet`; setup enables it again.

## Local Network Updates

Before applying an upgrade to a local network, the network should be stopped and reset with `dashmate reset --hard`.

## Notes on Compatibility

When upgrading between major versions, always check the release notes for any breaking changes or special upgrade instructions.
