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
  DASHMATE_CANDIDATE="$(apt-cache policy dashmate | awk '$1 == "Candidate:" { print $2 }')"
  if [ -n "$DASHMATE_INSTALLED" ] && [ -n "$DASHMATE_CANDIDATE" ] \
    && dpkg --compare-versions "$DASHMATE_CANDIDATE" gt "$DASHMATE_INSTALLED"; then
    if sudo apt install --only-upgrade --download-only "dashmate=$DASHMATE_CANDIDATE"; then
      DASHMATE_VERSION="$DASHMATE_CANDIDATE"
      echo "Downloaded dashmate $DASHMATE_VERSION"
    else
      echo "Could not download dashmate $DASHMATE_CANDIDATE. Leave the node running and try again later." >&2
    fi
  else
    echo "Nothing to update: apt offers no dashmate newer than ${DASHMATE_INSTALLED:-the installed one}. Leave the node running."
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
If the download step prints `Nothing to update`, there is no new dashmate to install, so leave the node running: a prerelease is offered only if you [follow prereleases](installation.md#prereleases).
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

* With the Debian package, run `dashmate stop` only after the download step printed `Downloaded`. If it printed `Nothing to update`, there is nothing to do: leave the node running.
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

## Major Updates

A major update installs and starts the same way as any other update, with no reset, unless the release notes say otherwise (see [Updates After a Platform Chain Restart](#updates-after-a-platform-chain-restart)).
Platform upgrades are protocol upgrades that the network activates, and dashmate applies any change to the configuration format itself through migrations when the new version first runs.
Follow the [example update process](#example-update-process) above: download the new Debian package before you stop the node, stop it, install the new version, check `dashmate --version`, run `dashmate update` and start the node.

## Updates After a Platform Chain Restart

A testnet or devnet may restart Platform from a new genesis, which starts its Platform chain again from block one. Mainnet Platform has not been restarted.
Only then is the node's old Platform data useless, and it has to be removed with `dashmate reset --platform`.
The reset asks for confirmation, then removes the Platform containers and the Drive and Tenderdash data, and keeps the configuration and the Core data.
Each testnet restart so far has shipped its new genesis in a dashmate release, as a configuration migration that applies when the new version first runs, so install that release before the reset.
A node that does not run Platform (`platform.enable` is `false`) can skip the reset, and dashmate refuses a Platform-only reset there anyway. If it ran Platform before, it still has that old Platform data, and it would use it if Platform were enabled again. To clear the data, enable Platform for the reset only: `dashmate config set platform.enable true`, then `dashmate reset --platform`, then `dashmate config set platform.enable false`.

As with any update, run the steps one at a time in the same shell, and stop at the first one that fails:

* With the Debian package, run `dashmate stop` only after the download step printed `Downloaded`. If it printed `Nothing to update`, leave the node running, unless `dashmate --version` already shows the release the restart needs: then skip the install step.
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

If the quorum list service is enabled (`platform.quorumList.enabled` is `true`), the Platform-only reset fails with `service "quorum_list" depends on undefined service "core"`, because that service depends on Core, which the reset leaves out. Turn the service off for the reset and back on before you start the node:

```bash
$ dashmate config set platform.quorumList.enabled false
$ dashmate reset --platform
$ dashmate config set platform.quorumList.enabled true
```

On a devnet, dashmate does not bring the new genesis. Set the genesis the devnet's operators publish with `dashmate config set platform.drive.tenderdash.genesis '<genesis JSON>'` before `dashmate start`.

### Do not add `--hard`

`dashmate reset --platform --hard` is not needed for a chain restart, and it removes things the node still needs:

* It sets the whole Platform part of the configuration back to the defaults. On an evonode this clears the Tenderdash node key, whose node ID is registered on the network for this evonode, and dashmate does not create a new one for a masternode: the key stays empty until you set it again.
* It also resets the gateway and SSL settings, and deletes the configuration's `platform` directory in the dashmate home directory (`~/.dashmate/<config name>/platform` by default), which holds the SSL certificate and its private key.
* A configuration with no defaults of its own (any name other than `mainnet` or `testnet`, outside the `local` group) has nothing to reset to, so the hard reset removes the whole configuration.

`dashmate setup` does not repair any of this: it refuses to run for a preset that is already configured, so it cannot be re-run on a configured node.
The reset it suggests instead, `dashmate reset --config <preset> --hard`, also removes the Core data and sets the preset's whole configuration, Core and Platform settings alike, back to the defaults, so it is not a way back for an evonode either.

## Local Network Updates

Before applying an upgrade to a local network, stop it with `dashmate group stop --group local` and reset it with `dashmate group reset --group local --hard`, which removes the local configurations. Name the group: without `--group`, both commands act on the current default group, which may not be `local`. Then set it up again with `dashmate setup local`.

## Notes on Compatibility

When upgrading between major versions, always check the release notes for any breaking changes or special upgrade instructions.
