# DAPI Documentation

This guide provides comprehensive information about DAPI, its architecture, configuration, and available endpoints.

## What is DAPI?

Historically, nodes in most cryptocurrency networks communicated with each other, and the outside world, according to a peer-to-peer (P2P) protocol.
The use of P2P protocols presented some downsides for developers, namely, network resources were difficult to access without specialized knowledge or trusted third-party services.

To overcome these obstacles, the Dash decentralized API (DAPI) uses Dash's robust masternode infrastructure to provide an API for accessing the network.
DAPI supports both layer 1 (Core blockchain) and layer 2 (Dash Platform) functionality so all developers can interact with Dash via a single interface.

DAPI offers several advantages over traditional centralized APIs:

- **No single point of failure** - Running on the masternode network ensures high availability
- **Censorship resistance** - No central authority can block access
- **Built-in scalability** - As the masternode network grows, API capacity grows
- **Multiple interfaces** - Supports both modern gRPC and legacy JSON-RPC

## DAPI Architecture

DAPI consists of two main processes:

1. **API Process** - Handles standard gRPC and JSON-RPC endpoints:
   - Core blockchain endpoints
   - Platform (Evolution) endpoints
   - Legacy JSON-RPC endpoints

2. **Core Streams Process** - Handles streaming data endpoints:
   - Block headers streaming
   - Transaction filtering and streaming
   - Masternode list updates

Each process connects independently to the underlying Dash infrastructure (Core, Drive, and Tenderdash) to provide its functionality.

Learn more about DAPI's architecture in the [Architecture](./architecture.md) section.

## Endpoints

### Overview
- [Endpoints Overview](./endpoints/index.md) - Index of all available API endpoints

### Core Endpoints
- [getBestBlockHeight](./endpoints/core/getBestBlockHeight.md)
- [getBlockchainStatus](./endpoints/core/getBlockchainStatus.md)
- [getTransaction](./endpoints/core/getTransaction.md)
- [broadcastTransaction](./endpoints/core/broadcastTransaction.md)
- [subscribeToMasternodeList](./endpoints/core/subscribeToMasternodeList.md)

### Platform Endpoints
- [broadcastStateTransition](./endpoints/platform/broadcastStateTransition.md)
- [waitForStateTransitionResult](./endpoints/platform/waitForStateTransitionResult.md)
- [getConsensusParams](./endpoints/platform/getConsensusParams.md)
- [getStatus](./endpoints/platform/getStatus.md)

### Stream Endpoints
- [subscribeToBlockHeadersWithChainLocks](./endpoints/streams/subscribeToBlockHeadersWithChainLocks.md)
- [subscribeToTransactionsWithProofs](./endpoints/streams/subscribeToTransactionsWithProofs.md)

### JSON-RPC Endpoints
- [getBestBlockHash](./endpoints/json-rpc/getBestBlockHash.md)
- [getBlockHash](./endpoints/json-rpc/getBlockHash.md)

## Client Libraries

Rather than directly interacting with DAPI's gRPC or JSON-RPC interfaces, most developers should use one of the official client libraries:

- [Dash SDK (JavaScript)](https://docs.dash.org/projects/platform/en/stable/docs/sdk-js/overview.html)
- [Dash SDK (Rust)](https://docs.dash.org/projects/platform/en/stable/docs/sdk-rs/overview.html) 
- [DAPI Client (JavaScript)](https://docs.dash.org/projects/platform/en/stable/docs/dapi-client-js/overview.html)
- [gRPC Clients](https://github.com/dashpay/platform/tree/master/packages/dapi-grpc) (for other languages)

These libraries handle the complexity of interacting with DAPI and provide idiomatic interfaces in their respective languages.
