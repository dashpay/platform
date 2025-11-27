# Quorum List service

The quorum list sidecar exposes the local LLMQ list over HTTP for SDKs and functional tests. It mirrors the public `quorums.*.networks.dash.org` endpoints but runs against your local Core node.

## When it runs

- **Optional**: disabled by default on all presets.
- **Local preset**: enabled automatically only on the `local_seed` node when you run `dashmate setup local`, so other local nodes stay unchanged.
- Controlled by `platform.quorumList.enabled` (profile `platform-quorum`).

## Image and ports

- Image: `dashpay/quorum-list-server:latest`
- API port: `platform.quorumList.api.port` (default `2444`)
- Host binding: `platform.quorumList.api.host` (default `127.0.0.1`)
- Container bind address is `0.0.0.0` inside the compose network.

## Core RPC access

- Uses the dedicated Core RPC user `quorum_list` (added to configs via migration).
- RPC URL points to the local Core container (`http://core:${CORE_RPC_PORT}`).

## Enabling manually

```bash
dashmate config set platform.quorumList.enabled true
dashmate start --platform   # or restart the node
```

## Compose service name

`quorum_list` (profile `platform-quorum`). It is only included when the toggle is on or when a command explicitly includes all platform profiles (e.g., platform-only reset/stop).
