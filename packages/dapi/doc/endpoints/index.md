# DAPI Endpoints Overview

DAPI offers a variety of endpoints through two main interfaces: gRPC and JSON-RPC. This document provides an overview of all available endpoints and links to detailed documentation.

## Interface Types

### gRPC (Recommended)

The gRPC interface is the recommended way to interact with DAPI. It offers:
- Better performance
- Strong typing
- Streaming capabilities
- Native support in many languages

### JSON-RPC (Legacy)

The JSON-RPC interface will eventually be deprecated. It offers:
- HTTP-based requests
- Compatibility with existing tools
- Simpler integration for basic use cases

## Endpoint Categories

DAPI endpoints are organized into three main categories:

### 1. Core Endpoints

These endpoints provide access to the underlying Dash blockchain (Core) functionality, such as blocks, transactions, and network status.

**Main gRPC endpoints:**
- [`getBestBlockHeight`](./core/getBestBlockHeight.md) - Returns the current blockchain height
- [`getBlockchainStatus`](./core/getBlockchainStatus.md) - Returns blockchain status information
- [`getTransaction`](./core/getTransaction.md) - Retrieves transaction data by ID
- [`broadcastTransaction`](./core/broadcastTransaction.md) - Broadcasts a raw transaction to the network
- [`subscribeToMasternodeList`](./core/subscribeToMasternodeList.md) - Stream masternode list updates

### 2. Platform Endpoints

These endpoints provide access to Dash Platform (Evolution) features, enabling interaction with decentralized applications, identities, and data contracts.

**Main gRPC endpoints:**
- [`broadcastStateTransition`](./platform/broadcastStateTransition.md) - Broadcasts a state transition to the platform
- [`waitForStateTransitionResult`](./platform/waitForStateTransitionResult.md) - Waits for a state transition to be processed
- [`getConsensusParams`](./platform/getConsensusParams.md) - Retrieves platform consensus parameters
- [`getStatus`](./platform/getStatus.md) - Gets platform status information

The following endpoints are defined in the gRPC service but are served by Drive ABCI directly:

- `getIdentity`
- `getIdentitiesContractKeys`
- `getIdentityBalance`
- `getIdentityBalanceAndRevision`
- `getIdentityKeys`
- `getDocuments`
- `getDataContract`
- `getDataContracts`
- `getDataContractHistory`
- `getIdentityByPublicKeyHash`
- `getIdentitiesByPublicKeyHashes`
- `getProofs`
- `getEpochsInfo`
- `getProtocolVersionUpgradeVoteStatus`
- `getProtocolVersionUpgradeState`
- `getIdentityContractNonce`
- `getIdentityNonce`


### 3. Stream Endpoints

These endpoints provide real-time streaming data from the Dash network, including blocks, transactions, and masternode list updates.

**Main streaming endpoints:**
- [`subscribeToBlockHeadersWithChainLocks`](./streams/subscribeToBlockHeadersWithChainLocks.md) - Stream block headers and chain locks
- [`subscribeToTransactionsWithProofs`](./streams/subscribeToTransactionsWithProofs.md) - Stream transactions matching a bloom filter

### 4. JSON-RPC Endpoints

These endpoints provide a subset of Dash Core functionality through the JSON-RPC interface for backward compatibility.

**Available endpoints:**
- [`getBestBlockHash`](./json-rpc/getBestBlockHash.md) - Returns the hash of the best block
- [`getBlockHash`](./json-rpc/getBlockHash.md) - Returns the hash of a block at a specific height

## Using the Endpoints

### Direct Access

You can access these endpoints directly using gRPC or JSON-RPC clients:

**gRPC Example (using grpcurl):**
```bash
grpcurl -plaintext localhost:2500 org.dash.platform.dapi.v0.Core/getBestBlockHeight
```

**JSON-RPC Example (using curl):**
```bash
curl -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"getBestBlockHash","params":[],"id":1}' http://localhost:2501
```

### Client Libraries

For most applications, it's recommended to use one of the DAPI client libraries:

- [Dash SDK (JavaScript)](https://docs.dash.org/projects/platform/en/stable/docs/sdk-js/overview.html)
- [Dash SDK (Rust)](https://docs.dash.org/projects/platform/en/stable/docs/sdk-rs/overview.html)
- [DAPI Client (JavaScript)](https://docs.dash.org/projects/platform/en/stable/docs/dapi-client-js/overview.html)
- [gRPC Clients](https://github.com/dashpay/platform/tree/master/packages/dapi-grpc) (for other languages)

These client libraries handle the complexity of interacting with DAPI and provide a more convenient interface for developers.
