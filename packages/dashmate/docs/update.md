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

On Debian and Ubuntu, update dashmate with apt from Dash's apt repository. dashmate packages from 5.1.0-rc.2 on add the repository when they are installed, so it is configured whether you installed dashmate from the repository or from a downloaded package (see [A host without the repository](#a-host-without-the-repository) for older ones).
Download the new package before you stop the node, so a failed download never leaves the node stopped. This block refreshes apt's indexes, compares the installed version with the one apt would install, and downloads that version only if it is newer:

```bash
DASHMATE_VERSION=""
if sudo apt update --error-on=any; then
  DASHMATE_INSTALLED="$(dpkg-query -W -f='${Version}' dashmate)"
  DASHMATE_CANDIDATE="$(LC_ALL=C apt-cache policy dashmate | awk '$1 == "Candidate:" { print $2 }')"
  if [ -n "$DASHMATE_INSTALLED" ] && [ -n "$DASHMATE_CANDIDATE" ] \
    && dpkg --compare-versions "$DASHMATE_CANDIDATE" gt "$DASHMATE_INSTALLED"; then
    if sudo apt install --only-upgrade --download-only "dashmate=$DASHMATE_CANDIDATE"; then
      DASHMATE_VERSION="$DASHMATE_CANDIDATE"
      echo "Downloaded dashmate $DASHMATE_VERSION"
    else
      echo "Could not download dashmate $DASHMATE_CANDIDATE. Leave the node running and try again later." >&2
    fi
  else
    echo "No new dashmate package: apt offers none newer than ${DASHMATE_INSTALLED:-the installed one}. Leave the node running and skip the install step."
  fi
else
  echo "apt update failed. Leave the node running, fix the error and try again." >&2
fi
```

apt keeps the downloaded package in its cache. After stopping the node with `dashmate stop` (see the examples below), install that version in the same shell. `--no-download` makes apt use the cached package only, so the install never downloads anything while the node is stopped:

```bash
if [ -n "$DASHMATE_VERSION" ]; then
  sudo apt install --only-upgrade --no-download "dashmate=$DASHMATE_VERSION"
else
  echo "No downloaded dashmate version found, so nothing was installed. Run the download step again in this shell." >&2
fi
```

Use these commands rather than `sudo apt upgrade`, which installs the new dashmate as well, but while the node is still running.
If the download step prints `No new dashmate package`, there is no dashmate package to install, so leave the node running and skip the install step: a prerelease is offered only if you [follow prereleases](installation.md#prereleases). The component images can still have updates, so go on with `dashmate update` (see the [example update process](#example-update-process)).
Without `--error-on=any`, `apt update` succeeds even when it cannot reach a repository and goes on with the indexes it already has, so the block could miss a new release or report one apt can no longer download. With it, an error in any configured repository stops the block, so fix or disable a failing repository of another vendor before you update.

##### A host without the repository

dashmate packages older than 5.1.0-rc.2 do not add the repository, so on a host that runs one, `/etc/apt/sources.list.d/dashmate.sources` does not exist and apt finds no new dashmate.
Add the repository once with the [commands in the installation guide](installation.md#debian-and-ubuntu-dashs-apt-repository), which end with `sudo apt update` and install nothing, then update as above.
You can also download the new package from the GitHub releases page with the [download block in the installation guide](installation.md#debian-package-from-the-github-releases-page) before you stop the node, and install it after `dashmate stop` with the [install block](installation.md#installing-the-downloaded-package). It adds the repository, so later updates come through apt.

#### Node package

If you installed dashmate with NPM, install the new version after stopping the node with `dashmate stop`:

```bash
npm install -g dashmate
```

### Example update process:

Run the steps one at a time in the same shell, and stop at the first one that fails:

* With the Debian package, run `dashmate stop` only after the download step printed `Downloaded`.
* Before `dashmate update`, check that `dashmate --version` shows the new version.
* If a step after `dashmate stop` fails, run `dashmate start` to bring the node back on the version it has, then fix the problem and start over.
* If the download step printed `No new dashmate package`, there is no package to install: leave the node running, skip `dashmate stop`, the install step and the version check, and run `dashmate update`. If it shows `updated` for any service, run `dashmate restart` so the node uses the new images; if every service is `up to date`, there is nothing to restart.

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

## Major Updates

A major update installs and starts the same way as any other update, with no reset, unless the release notes say otherwise (see [Updates After a Platform Chain Restart](#updates-after-a-platform-chain-restart)).
Platform upgrades are protocol upgrades that the network activates, and dashmate applies any change to the configuration format itself through migrations when the new version first runs.
Follow the [example update process](#example-update-process) above: download the new Debian package before you stop the node, stop it, install the new version, check `dashmate --version`, run `dashmate update` and start the node.

## Updates After a Platform Chain Restart

A testnet or devnet may restart Platform from a new genesis, which starts its Platform chain again from block one. Mainnet Platform has not been restarted.
Only then is the node's old Platform data useless, and it has to be removed with `dashmate reset --platform`.
The reset asks for confirmation, then removes the Platform containers and the Drive and Tenderdash data, and keeps the configuration and the Core data.
Each testnet restart so far has shipped its new genesis in a dashmate release, as a configuration migration that applies when the new version first runs, so install that release before the reset.
A node that does not run Platform (`platform.enable` is `false`) can skip the reset. If it ran Platform before, it still has that old Platform data, and it would use it if Platform were enabled again. `dashmate reset --platform` clears that data on such a node too, and leaves its configuration as it is.

As with any update, run the steps one at a time in the same shell, and stop at the first one that fails:

* With the Debian package, run `dashmate stop` only after the download step printed `Downloaded`. If it printed `No new dashmate package`, leave the node running, unless `dashmate --version` already shows the release the restart needs: then skip the install step.
* Check that `dashmate --version` shows the new version before you reset Platform data.
* If a step fails before `dashmate reset --platform`, nothing has been removed: run `dashmate start` to bring the node back on the version it has, then fix the problem and start over.
* If `dashmate reset --platform` or a later step fails, fix the problem, run that step again and finish the remaining steps in order. Running the reset again is safe. The node starts with an empty Platform chain, which is what the restarted network needs.

```bash
# Debian package only: download the new package first (see above)
$ dashmate stop
# Install the new dashmate version (Debian package or NPM, see above)
$ dashmate --version
$ dashmate reset --platform
$ dashmate update
$ dashmate start
```

On a devnet, dashmate does not bring the new genesis. Set the genesis the devnet's operators publish with `dashmate config set platform.drive.tenderdash.genesis '<genesis JSON>'` before `dashmate start`.

### Do not add `--hard`

`dashmate reset --platform --hard` is not needed for a chain restart, and it removes things the node still needs:

* It sets the whole Platform part of the configuration back to the defaults. On an evonode this clears the Tenderdash node key, whose node ID is registered on the network for this evonode, and dashmate does not create a new one for a masternode: the key stays empty until you set it again.
* It also resets the gateway and SSL settings, and deletes the configuration's `platform` directory in the dashmate home directory (`~/.dashmate/<config name>/platform` by default), which holds the SSL certificate and its private key.

Plain `dashmate setup` does not repair any of this: it refuses to run for a preset that is already configured, so it cannot be re-run on a configured node.
The reset it suggests instead, `dashmate reset --config <preset> --hard`, also removes the Core data and sets the preset's whole configuration, Core and Platform settings alike, back to the defaults, so it is not a way back for an evonode either.
The way back is `dashmate setup --platform`, which sets up only the Platform part again. An evonode needs the node key it had, so back it up before the hard reset: see [Starting Platform Over](#starting-platform-over).

A configuration with no defaults of its own (any name other than `mainnet` or `testnet`, outside the `local` group) has nothing to reset its Platform options to, so dashmate refuses the hard Platform reset there and changes nothing.

## Starting Platform Over

`dashmate reset --platform --hard` sets every Platform option back to the preset's defaults: the node key, the Platform ports, the certificate provider and the certificate. Nothing of Platform survives it. On mainnet it also disables Platform, because the mainnet preset does. Set Platform up again with `dashmate setup --platform`, which configures only Platform and leaves Core as it is.

An evonode's node key is bound to its registration (the ProRegTx or a later ProUpServTx records its node ID), and the hard reset keeps no copy of it, so the flow starts with a backup into a new directory in your home directory, named after the config, that only you can open. `mkdir` refuses a directory or file of that name that exists, so an earlier backup is never lost or written into; a failed export removes what it wrote, so the command can run again; and the parentheses keep the `umask` from staying set in your shell. Check that the backup is not empty before you reset, and keep it in that directory or another one only you can open:

```bash
$ dashmate stop --platform --config testnet            # skip if Platform is not running
$ (umask 077 && mkdir -m 700 "$HOME"/node-key-testnet && set -o noclobber && { dashmate config get platform.drive.tenderdash.node.key --config testnet > "$HOME"/node-key-testnet/node-key || { rm -f "$HOME"/node-key-testnet/node-key; rmdir "$HOME"/node-key-testnet; exit 1; }; })
$ dashmate reset --platform --hard --config testnet
$ dashmate setup --platform --config testnet --platform-node-key-file "$HOME"/node-key-testnet/node-key
$ dashmate update --config testnet                     # now pulls the Platform images
$ dashmate start --platform --config testnet           # or `dashmate start --config testnet` if Core is stopped
```

`setup --platform` reads the registration from the masternode list, through Core, and accepts only the key whose node ID is registered. If Core is stopped it starts Core for the duration of the command and waits until Core has synced. It also obtains the gateway certificate.

Run setup before `update`. On mainnet the reset disables Platform, so an earlier `update` pulls no Platform images, and `dashmate start` runs Core alone without saying so: nothing reminds you that Platform still has to be set up. On testnet an earlier `update` says that Platform must be set up first and skips its certificate check, and `start`, `restart` and `core reindex` refuse to start Platform, on a masternode and on an Evo fullnode, until it is set up. To run Core alone in the meantime, disable Platform with `dashmate config set platform.enable false --config testnet`; setup enables it again.

If `platform.quorumList.enabled` is `true`, stop the whole node with `dashmate stop --config testnet` instead: `stop --platform` also stops Core then.

To combine this with a dashmate upgrade, do the normal upgrade first (stop the node, install the new version, check `dashmate --version`), then continue from the backup above; Platform is already stopped.

Unattended, with the key read from stdin rather than the command line:

```bash
$ dashmate setup --platform testnet --non-interactive --ssl-provider letsencrypt \
    --platform-node-key-file - < "$HOME"/node-key-testnet/node-key
```

See [setup](./commands/setup.md#platform-only-setup) for every flag.

### Lost node key

dashmate cannot recover a deleted node key, and it does not change a registration. Register a new key first, then set Platform up with it:

1. Generate a key in DMT, or with `tenderdash gen-node-key` where Tenderdash is installed: it prints a `node_key.json`; redirect it into a file only you can read.
2. Submit a ProUpServTx with the new node ID and the existing endpoints: DMT "Update service", or `protx update_service_evo` in the Dash-Qt console. Its third argument is the operator BLS private key, so do not paste it into a shell. Update the DMT entry too, or a later DMT "Update service" sends the old node ID again.
3. Wait until the transaction is mined, then run `dashmate setup --platform --config testnet --platform-node-key-file <file>`. The check confirms the new registration.

Until then dashmate refuses to start Platform on the node. To run Core alone in the meantime, disable Platform with `dashmate config set platform.enable false --config testnet`; setup enables it again.

## Local Network Updates

Before applying an upgrade to a local network, stop it with `dashmate group stop --group local` and reset it with `dashmate group reset --group local --hard`, which removes the local configurations. Name the group: without `--group`, both commands act on the current default group, which may not be `local`. Then set it up again with `dashmate setup local`.

## Notes on Compatibility

When upgrading between major versions, always check the release notes for any breaking changes or special upgrade instructions.
