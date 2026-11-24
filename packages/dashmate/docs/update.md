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

### Installing the new dashmate version

Install the new version of dashmate the same way you installed it originally (see [Installation](installation.md)), after stopping the node with `dashmate stop` as shown in the example below.

#### Debian package

If you installed dashmate from the Debian package, download the newest dashmate installation package for your architecture from the [GitHub releases page](https://github.com/dashpay/platform/releases/latest).
The file name contains the version, so it changes with every release; this downloads the one matching the architecture you are on:

```bash
DASHMATE_DEB="$(curl -fsSL https://api.github.com/repos/dashpay/platform/releases/latest \
  | grep -o "https://[^\"]*_$(dpkg --print-architecture)\.deb" \
  | head -n 1)"
if [ -z "$DASHMATE_DEB" ]; then
  echo "Could not find a dashmate .deb for this architecture in the latest release" >&2
else
  curl -fLO "$DASHMATE_DEB"
fi
```

Install the package that was just downloaded:

```bash
sudo apt update
sudo apt install "./$(basename "$DASHMATE_DEB")"
```

> **Note:** At the end of the installation process, apt may display an error due to installing a downloaded package.
> You can ignore this error message:
> N: Download is performed unsandboxed as root as file '/home/ubuntu/dashmate_5.1.0-1_amd64.deb' couldn't be accessed by user '_apt'. - pkgAcquire::Run (13: Permission denied)

#### Node package

If you installed dashmate with NPM, install the new version with:

```bash
npm install -g dashmate
```

### Example update process:

```bash
$ dashmate stop
# Install the new dashmate version (Debian package or NPM, see above)
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

In some cases, you must also additionally reset platform data:

* When the upgrade contains non-compatible changes (e.g. switching between v22/v23)
* When the `dashmate setup` command exited with errors or was interrupted
* When the platform layer was wiped on the network

The reset and update process for major updates:

```bash
$ dashmate stop
# Install the new dashmate version (Debian package or NPM, see above)
$ dashmate reset --platform --hard
$ dashmate update
$ dashmate setup
$ dashmate start
```

## Local Network Updates

Before applying an upgrade to a local network, the network should be stopped and reset with `dashmate reset --hard`.

## Notes on Compatibility

When upgrading between major versions, always check the release notes for any breaking changes or special upgrade instructions.
