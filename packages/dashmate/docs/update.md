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

Install the new version of dashmate the same way you installed it originally (see [Installation](installation.md)).

#### Debian package

<!-- The Debian download and install blocks below also appear in installation.md. Keep both copies identical. -->

If you installed dashmate from the Debian package, download the newest dashmate installation package for your architecture from the [GitHub releases page](https://github.com/dashpay/platform/releases/latest).
This installs stable releases only: the snippet below reads `releases/latest`, which leaves prereleases out.
If you run a prerelease, use the [prerelease download](#downloading-a-prerelease-package) below instead.

The file name contains the version, so it changes with every release; this downloads the one matching the architecture you are on into a new private directory, so no other user can replace it before it is installed.
Download it before you stop the node, so a failed download never leaves the node stopped:

```bash
DASHMATE_DEB=""
DASHMATE_DEB_URL="$(curl -fsSL https://api.github.com/repos/dashpay/platform/releases/latest \
  | grep -o "https://[^\"]*_$(dpkg --print-architecture)\.deb" \
  | head -n 1)"
DASHMATE_DEB_DIR="$(mktemp -d)"
if [ -n "$DASHMATE_DEB_URL" ] && [ -n "$DASHMATE_DEB_DIR" ] && [ -d "$DASHMATE_DEB_DIR" ] \
  && curl -fL -o "$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")" "$DASHMATE_DEB_URL"; then
  DASHMATE_DEB="$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")"
  echo "Downloaded $DASHMATE_DEB"
else
  echo "Could not download the dashmate package. Try again later." >&2
fi
```

##### Downloading a prerelease package

To install a prerelease, open that release's page under [GitHub releases](https://github.com/dashpay/platform/releases) and copy the link to the `.deb` file for your architecture (`dpkg --print-architecture` prints it).
Download it with this block instead of the one above, replacing the example link with the one you copied.
As with a stable release, download it before you stop the node:

```bash
DASHMATE_DEB=""
DASHMATE_DEB_URL="https://github.com/dashpay/platform/releases/download/v5.1.0-dev.3/dashmate_5.1.0.dev.3-1_amd64.deb"
DASHMATE_DEB_DIR="$(mktemp -d)"
if [ -n "$DASHMATE_DEB_DIR" ] && [ -d "$DASHMATE_DEB_DIR" ] \
  && curl -fL -o "$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")" "$DASHMATE_DEB_URL"; then
  DASHMATE_DEB="$DASHMATE_DEB_DIR/$(basename "$DASHMATE_DEB_URL")"
  echo "Downloaded $DASHMATE_DEB"
else
  echo "Could not download the dashmate package. Check the link and try again." >&2
fi
```

##### Installing the downloaded package

After stopping the node with `dashmate stop` (see the examples below), install the package that either download block saved. It is installed by its path, and nothing is installed if the download failed:

```bash
if [ -n "$DASHMATE_DEB" ] && [ -f "$DASHMATE_DEB" ]; then
  sudo apt install "$DASHMATE_DEB"
else
  echo "No downloaded dashmate package found, so nothing was installed. Run the download step again in this shell." >&2
fi
```

> **Note:** At the end of the installation process, apt may display an error due to installing a downloaded package.
> You can ignore this error message:
> N: Download is performed unsandboxed as root as file '/tmp/tmp.k3Jd8sQw2L/dashmate_5.1.0-1_amd64.deb' couldn't be accessed by user '_apt'. - pkgAcquire::Run (13: Permission denied)

#### Node package

If you installed dashmate with NPM, install the new version after stopping the node with `dashmate stop`:

```bash
npm install -g dashmate
```

### Example update process:

Run the steps one at a time in the same shell, and stop at the first one that fails:

* With the Debian package, run `dashmate stop` only after the download printed `Downloaded ...`.
* Before `dashmate update`, check that `dashmate --version` shows the new version.
* If a step after `dashmate stop` fails, run `dashmate start` to bring the node back on the version it has, then fix the problem and start over.

```bash
# Debian package only: download the new package first (see above)
$ dashmate stop
# Install the new dashmate version (Debian package or NPM, see above)
$ dashmate --version
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

The reset and update process for major updates also runs one step at a time and stops at the first one that fails.
Check that `dashmate --version` shows the new version before you reset platform data.

What to do after a failure depends on whether the reset has run:

* Before `dashmate reset --platform --hard`, no Platform data has been removed: `dashmate start` brings the node back up on the dashmate version now installed.
* After it, `dashmate start` does not bring the old node back. The reset deletes Platform data, and nothing restores it. What to run next depends on what the reset did to the configuration:
  * A configuration that has defaults (the `mainnet` and `testnet` presets, and the `local` group) is reset in place: its Platform part goes back to the defaults, and the configuration still exists. Fix the problem and finish the remaining steps in order: `dashmate update`, then `dashmate setup` to configure Platform again, then `dashmate start`.
  * A configuration with any other name has no defaults to reset to, so the reset removes it, and `dashmate update` refuses to run without a configuration. Run `dashmate setup` first to create a configuration again, then `dashmate update` and `dashmate start`.

The process:

```bash
# Debian package only: download the new package first (see above)
$ dashmate stop
# Install the new dashmate version (Debian package or NPM, see above)
$ dashmate --version
$ dashmate reset --platform --hard
# If the reset removed the configuration (see above), run dashmate setup before dashmate update
$ dashmate update
$ dashmate setup
$ dashmate start
```

## Local Network Updates

Before applying an upgrade to a local network, the network should be stopped and reset with `dashmate reset --hard`.

## Notes on Compatibility

When upgrading between major versions, always check the release notes for any breaking changes or special upgrade instructions.
