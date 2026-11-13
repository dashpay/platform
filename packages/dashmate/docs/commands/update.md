# update

The `update` command updates the node's Docker images to their latest versions.

## Usage

```bash
dashmate update [OPTIONS]
```

## Options

| Option | Description | Default |
|--------|-------------|--------|
| `-c, --config=<name>` | Configuration to update | *Uses default config if not specified* |
| `--format=<format>` | Display output format (`plain`, `json`, `yaml`) | `plain` |
| `--skip-image-cleanup` | Keep unused historical Dash Docker images | `false` |

## Description

This command updates the Docker images used by a Dash node to their latest versions.
It pulls the latest images specified in the configuration and reports on which services were updated.

The command displays a table showing:
- Service names
- Docker image identifiers
- Update status (updated, up to date, or error)

Services that were updated are highlighted in yellow, services that were already up to date are shown in green, and any errors are displayed in red.

This command only updates the Docker images; it does not automatically restart services to use the new images.
After updating, you typically need to restart the node with the `restart` command to apply the updates.

### Historical Docker image cleanup

After all image pulls succeed and the certificate check permits the update,
dashmate removes unused first-party Dash images **built more than 14 days ago**.
It discovers existing images directly from Docker, so the first run can reclaim
images accumulated over previous years; no tracking history is required.

Cleanup recognizes current and historical Docker Hub repositories used by
dashmate: `dashpay/dashd`, `dashpay/drive`, `dashpay/dapi`, `dashpay/rs-dapi`,
`dashpay/tenderdash`, `dashpay/dashmate-helper`, `dashpay/envoy`,
`dashpay/quorum-list-server`, `dashpay/insight`, and `dashpay/insight-api`.
Fully untagged Drive, DAPI and helper images can also be identified by their
specific historical build labels. Unidentifiable images, third-party images,
custom/local repositories and images with unrelated aliases are left alone.

The following images are always retained:

- Images referenced by any running or stopped container, including other applications.
- Images configured in any configuration in this dashmate home, including disabled services.
- Newly pulled images and the immediate pre-update images needed for rollback during this update.
- Images built within the last 14 days.

The age is the image's **build date**, not when it was downloaded or last used.
This is not a guarantee of 14 days of rollback retention after an upgrade.
Images still attached to old containers become eligible on a later update after
those containers have been replaced. Volumes, containers, networks and build
cache are not pruned, and image removal is never forced.

Cleanup uses only the local Docker API and does not contact Docker Hub. If any
pull fails (for example, the internet or Docker Hub is unavailable), no cleanup
is performed. A stalled pull still follows the existing update behavior: there
is no separate dashmate-level pull deadline, and cleanup waits for all pulls.
Cleanup problems are reported as warnings without failing an otherwise
successful update. Diagnostics go to stderr, preserving the JSON result on stdout.

Use `dashmate update --skip-image-cleanup` to retain all existing images.

## Examples

```bash
# Update the default configuration
dashmate update

# Update a specific configuration
dashmate update --config=testnet

# Update and display results in JSON format
dashmate update --format=json
```

Example output:
```
┌─────────────────┬──────────────────────────────┬────────────┐
│ Service         │ Image                        │ Updated    │
├─────────────────┼──────────────────────────────┼────────────┤
│ Core            │ dashpay/dashd:19.2.0        │ up to date │
│ Drive ABCI      │ dashpay/drive:1.0.0         │ updated    │
│ Tenderdash      │ dashpay/tenderdash:0.10.0   │ updated    │
│ DAPI API        │ dashpay/dapi:1.0.0          │ up to date │
│ Envoy Gateway   │ dashpay/platform-gateway:1.0 │ up to date │
└─────────────────┴──────────────────────────────┴────────────┘
```

## Related Commands

- [restart](./restart.md) - Restart node to apply updates
- [docker build](./docker/build.md) - Build custom Docker images
