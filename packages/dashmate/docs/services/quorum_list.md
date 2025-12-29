# Quorum List service

The quorum list sidecar exposes the local LLMQ list over HTTP for SDKs and functional tests. It mirrors the public `quorums.*.networks.dash.org` endpoints but runs against your local Core node.

## When it runs

- **Optional**: disabled by default on all presets.
- **Local preset**: enabled automatically only on the `local_seed` node when you run `dashmate setup local`, so other local nodes stay unchanged.
- Controlled by `platform.quorumList.enabled` (profile `platform-quorum`).

## Configuration options

| Setting                                    | Default                            | Description                                                      |
|--------------------------------------------|------------------------------------|------------------------------------------------------------------|
| `platform.quorumList.enabled`              | `false`                            | Enable the quorum list service                                   |
| `platform.quorumList.docker.image`         | `dashpay/quorum-list-server:latest`| Docker image to use                                              |
| `platform.quorumList.api.host`             | `127.0.0.1`                        | Host interface to bind the API                                   |
| `platform.quorumList.api.port`             | `2444`                             | Port for the HTTP API                                            |
| `platform.quorumList.previousBlocksOffset` | `8`                                | Number of previous blocks to consider for quorum lookups         |
| `platform.quorumList.versionCheckHost`     | `""`                               | Host for version checking (set to `host.docker.internal` locally)|
| `platform.quorumList.addressHostOverride`  | `""`                               | Override for address host (set to `127.0.0.1` locally)           |

## Image and ports

- Image: `dashpay/quorum-list-server:latest`
- API port: `platform.quorumList.api.port` (default `2444`)
- Host binding: `platform.quorumList.api.host` (default `127.0.0.1`)
- Container bind address is `0.0.0.0` inside the compose network.

## Core RPC access

- Uses the dedicated Core RPC user `quorum_list` (added to configs via migration).
- RPC URL points to the local Core container (`http://core:${CORE_RPC_PORT}`).
- Whitelisted RPC commands: `quorum`, `masternode`, `getblockcount`

## Enabling manually

```bash
dashmate config set platform.quorumList.enabled true
dashmate start --platform   # or restart the node
```

## Compose service name

`quorum_list` (profile `platform-quorum`). It is only included when the toggle is on or when a command explicitly includes all platform profiles (e.g., platform-only reset/stop).

## Local network usage

### How SDKs use the quorum list

When running functional tests or developing against a local dashmate network, SDKs need access to quorum public keys to verify proofs. The quorum list service provides this via HTTP.

In the WASM SDK, before building a trusted client for local development:

```javascript
import { init } from '@dashevo/wasm-sdk';
import * as sdk from '@dashevo/wasm-sdk';

await init();

// Prefetch quorum information from local quorum list service
// Uses http://127.0.0.1:2444 by default
await sdk.WasmSdk.prefetchTrustedQuorumsLocal();

// Build a trusted client for local network
const builder = sdk.WasmSdkBuilder.local();
const client = builder.build();
```

### Local network setup

When running `dashmate setup local`, the quorum list service is automatically enabled on the `local_seed` node with the following configuration:

- `platform.quorumList.enabled` = `true`
- `platform.quorumList.versionCheckHost` = `host.docker.internal`
- `platform.quorumList.addressHostOverride` = `127.0.0.1`

The `versionCheckHost` and `addressHostOverride` settings are specific to local networks where the container needs to communicate with the host machine. For non-local networks, these are left empty.

The service:

1. Connects to the local Core node via RPC
2. Fetches active quorum information using `quorum listextended` and `quorum info`
3. Retrieves masternode list with `masternode list`
4. Exposes this data over HTTP at `http://127.0.0.1:2444`

### Verifying the service is running

```bash
# Check if the quorum_list container is running
docker ps | grep quorum_list

# Test the API endpoint
curl http://127.0.0.1:2444/quorums

# View service logs
docker logs <quorum_list_container_id>
```

### Troubleshooting

**Service not starting:**

- Ensure `platform.quorumList.enabled` is `true` on the seed node
- Check that the Core node is fully synced
- Verify the Core RPC user `quorum_list` exists in the config

**SDK cannot connect:**

- Verify port 2444 is accessible from the host
- Check that `platform.quorumList.api.host` allows connections from your client
- Ensure no firewall is blocking the port

**Quorum data not available:**

- Wait for the local network to establish quorums (may take a few blocks after initial setup)
- Check Core node logs for quorum-related errors
