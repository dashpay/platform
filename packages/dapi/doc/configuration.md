# DAPI Configuration

This document describes how to configure DAPI for different environments and use cases.

## Configuration Options

DAPI can be configured through environment variables. Here are the available configuration options:

| Environment Variable | Description | Default Value |
|----------------------|-------------|---------------|
| LIVENET | Set to 'true' for live network operation | false |
| API_JSON_RPC_PORT | Port for JSON-RPC server | 2501 |
| API_GRPC_PORT | Port for gRPC server | 2500 |
| TX_FILTER_STREAM_GRPC_PORT | Port for transaction filter stream gRPC server | 2510 |
| DASHCORE_RPC_PROTOCOL | Protocol for Dash Core RPC | http |
| DASHCORE_RPC_USER | Username for Dash Core RPC | dashrpc |
| DASHCORE_RPC_PASS | Password for Dash Core RPC | password |
| DASHCORE_RPC_HOST | Host for Dash Core RPC | 127.0.0.1 |
| DASHCORE_RPC_PORT | Port for Dash Core RPC | 30002 |
| DASHCORE_ZMQ_HOST | Host for Dash Core ZMQ | 127.0.0.1 |
| DASHCORE_ZMQ_PORT | Port for Dash Core ZMQ | 30003 |
| DASHCORE_P2P_HOST | Host for Dash Core P2P | 127.0.0.1 |
| DASHCORE_P2P_PORT | Port for Dash Core P2P | 30001 |
| DASHCORE_P2P_NETWORK | Network for Dash Core P2P | testnet |
| DRIVE_RPC_HOST | Host for Drive RPC | 127.0.0.1 |
| DRIVE_RPC_PORT | Port for Drive RPC | 6000 |
| BLOCK_HEADERS_CACHE_SIZE | Size of block headers cache | 500 |
| NETWORK | Network to connect to | testnet |
| BLOOM_FILTER_PERSISTENCE_TIMEOUT | Timeout for bloom filter persistence (ms) | 60000 |
| TENDERMINT_RPC_HOST | Host for Tendermint RPC | undefined |
| TENDERMINT_RPC_PORT | Port for Tendermint RPC | undefined |

## Setting Up Dependencies

DAPI requires the following dependencies to operate correctly:

### Dash Core

Dash Core is required for DAPI to access the Dash blockchain. You need to configure ZMQ for the following topics:

```
zmqpubhashblock=tcp://0.0.0.0:30003
zmqpubhashtx=tcp://0.0.0.0:30003
zmqpubrawblock=tcp://0.0.0.0:30003
zmqpubrawtx=tcp://0.0.0.0:30003
zmqpubhashchainlock=tcp://0.0.0.0:30003
zmqpubrawchainlock=tcp://0.0.0.0:30003
zmqpubhashgovernancevote=tcp://0.0.0.0:30003
zmqpubhashgovernanceobject=tcp://0.0.0.0:30003
zmqpubrawgovernancevote=tcp://0.0.0.0:30003
zmqpubhashinstantsenddoublesend=tcp://0.0.0.0:30003
zmqpubrawinstantsenddoublesend=tcp://0.0.0.0:30003
zmqpubhashinstantsendsuccess=tcp://0.0.0.0:30003
zmqpubrawinstantsendsuccess=tcp://0.0.0.0:30003
zmqpubrawtxlocksig=tcp://0.0.0.0:30003
```

You can find a sample configuration for Dash Core in [doc/dependencies_configs/dash.conf](./dependencies_configs/dash.conf).

### Drive

Drive is required for DAPI to interact with Dash Platform's state. Configure the Drive RPC connection using the following environment variables:

```
DRIVE_RPC_HOST=127.0.0.1
DRIVE_RPC_PORT=6000
```

### Tenderdash

Tenderdash is the consensus engine for Dash Platform. Configure the Tenderdash connection using:

```
TENDERMINT_RPC_HOST=127.0.0.1
TENDERMINT_RPC_PORT=26657
```

## Running DAPI

DAPI runs two separate processes:

1. **API Process**: Handles gRPC and JSON-RPC requests
   ```
   node scripts/api.js
   ```

2. **Core Streams Process**: Handles streaming data
   ```
   node scripts/core-streams.js
   ```

Both processes should be running for DAPI to function correctly. For more details about these processes, see the [Architecture](./architecture.md) document.

## Production Configuration

For production environments, it's recommended to:

1. Use a process manager like PM2 to ensure processes stay running
2. Set `NODE_ENV=production` to optimize error handling
3. Use HTTPS for production deployments
4. Configure proper security measures (firewalls, authentication)

Example PM2 configuration:

```json
{
  "apps": [
    {
      "name": "dapi-api",
      "script": "scripts/api.js",
      "env": {
        "NODE_ENV": "production",
        "LIVENET": "true"
      }
    },
    {
      "name": "dapi-core-streams",
      "script": "scripts/core-streams.js",
      "env": {
        "NODE_ENV": "production",
        "LIVENET": "true"
      }
    }
  ]
}
```

## Next Steps

- Learn more about DAPI's architecture in the [Architecture](./architecture.md) document
- Explore available endpoints in the [Endpoints](./endpoints/index.md) section
- Get started with DAPI using the [Getting Started](./getting-started.md) guide