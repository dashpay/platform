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
$ dashmate update --format=json 
[{"name":"core","title":"Core","updated":false,"image":"dashpay/dashd:19.2.0"},{"name":"drive_abci","title":"Drive ABCI","pulled":false,"image":"dashpay/drive:0.24"},{"name":"drive_tenderdash","title":"Drive Tenderdash","pulled":true,"image":"dashpay/tenderdash:0.11.2"},{"name":"rs_dapi","title":"rs-dapi","pulled":false,"image":"dashpay/rs-dapi:0.24"},{"name":"gateway","title":"Gateway","pulled":false,"image":"dashpay/envoy:0.24"},{"name":"dashmate_helper","title":"Dashmate Helper","pulled":false,"image":"dashpay/dashmate-helper:0.24"}]
```

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
