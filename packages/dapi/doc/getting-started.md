# Getting Started with DAPI

This guide will help you set up DAPI, understand its dependencies, and make your first API requests.

## Installation

To install DAPI, follow these steps:

```sh
# Clone the repository (if you haven't already)
git clone https://github.com/dashpay/platform.git
cd platform/packages/dapi

# Install dependencies
npm install
```

## Dependencies

DAPI requires several dependencies to function properly:

### 1. Node.js

DAPI targets the latest v20 release of Node.js.

### 2. Dash Core

DAPI requires the latest version of [Dash Core](https://github.com/dashevo/dash-evo-branches/tree/evo) with Evolution features.

#### Installing Dash Core:

You can either:
- Use the Docker image: `dashcore:evo`
- Clone and build from [the repository](https://github.com/dashevo/dash-evo-branches/tree/evo)
  - Switch to the `evo` branch
  - Follow the [build instructions](https://github.com/dashevo/dash-evo-branches/tree/evo/doc)

#### Configuring Dash Core:

DAPI needs Dash Core's ZMQ interface to be exposed and all indexes enabled. You can find an example config for Dash Core [here](./dependencies_configs/dash.conf).

To start Dash Core with this config:
```sh
./src/dashd -conf=/path/to/your/dash.conf
```

### 3. Drive and Tenderdash

For Platform functionality, DAPI requires Drive and Tenderdash. The easiest way to set up a local development environment with all components is to use [Dash Platform Test Suite](https://github.com/dashevo/platform-test-suite).

## Configuration

DAPI is configured through environment variables, which can be passed directly or via a `.env` file. The main configuration options include:

- `API_JSON_RPC_PORT`: JSON-RPC server port (default: 2501)
- `API_GRPC_PORT`: gRPC server port (default: 2500)
- `TX_FILTER_STREAM_GRPC_PORT`: Transaction filter stream port (default: 2510)
- `DASHCORE_RPC_HOST`, `DASHCORE_RPC_PORT`, `DASHCORE_RPC_USER`, `DASHCORE_RPC_PASS`: Dash Core connection settings
- `DRIVE_RPC_HOST`, `DRIVE_RPC_PORT`: Drive connection settings
- `TENDERMINT_RPC_HOST`, `TENDERMINT_RPC_PORT`: Tenderdash connection settings

For a complete list of configuration options, see the [Configuration](./configuration.md) document.

## Running DAPI

DAPI consists of two separate processes:

```sh
# Start both processes using npm
npm start

# Or start them individually
node scripts/api.js         # API process
node scripts/core-streams.js # Core Streams process
```

Both processes should be running for DAPI to function correctly.

## Making API Requests

DAPI provides two interfaces: gRPC (recommended) and JSON-RPC (legacy).

### gRPC Examples

For gRPC, you can use client libraries in various languages or tools like [grpcurl](https://github.com/fullstorydev/grpcurl) for testing:

```bash
# Get blockchain status
grpcurl -plaintext localhost:2500 org.dash.platform.dapi.v0.Core/getBlockchainStatus

# Get best block height
grpcurl -plaintext localhost:2500 org.dash.platform.dapi.v0.Core/getBestBlockHeight

# Get Platform status
grpcurl -plaintext localhost:2500 org.dash.platform.dapi.v0.Platform/getStatus
```

### JSON-RPC Examples

For JSON-RPC, you can use any HTTP client, like `curl`:

```bash
# Get best block hash
curl -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"getBestBlockHash","params":[],"id":1}' http://localhost:2501

# Get block hash at height 1000
curl -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"getBlockHash","params":[1000],"id":1}' http://localhost:2501
```

## Using Client Libraries

For application development, we recommend using the DAPI client libraries rather than directly interfacing with DAPI:

### JavaScript

```js
const { DAPIClient } = require('@dashevo/dapi-client');
const client = new DAPIClient();

// Core API example
async function getBlockchainInfo() {
  const status = await client.core.getBlockchainStatus();
  console.log('Blockchain status:', status);
}

// Platform API example
async function getPlatformStatus() {
  const status = await client.platform.getStatus();
  console.log('Platform status:', status);
}
```

### Streaming Example

```js
const { DAPIClient } = require('@dashevo/dapi-client');
const client = new DAPIClient();

// Stream block headers
const stream = client.core.subscribeToBlockHeadersWithChainLocks();

stream.on('data', (response) => {
  if (response.rawBlockHeader) {
    console.log('Received block header');
  } else if (response.rawChainLock) {
    console.log('Received chain lock');
  }
});
```

## Next Steps

- Learn about [DAPI's architecture](./architecture.md)
- Explore the [available endpoints](./endpoints/index.md)
- Understand how to [configure DAPI](./configuration.md) for your needs
- Consult the [Dash Platform Developer Documentation](https://docs.dash.org/projects/platform/en/stable/) for more information about the broader Dash Platform ecosystem