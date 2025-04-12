# DAPI Overview

DAPI (Decentralized API) is the decentralized API for the Dash Evolution platform. It serves as a distributed and decentralized HTTP API that functions as the developers' access point to the Dash network.

## What is DAPI?

DAPI is an HTTP API that allows developers to interact with the Dash network in a simple and efficient way. By being a decentralized solution running on the masternode network, DAPI offers several advantages over traditional centralized APIs:

- No single point of failure
- Censorship resistance
- High availability
- Built-in scalability

DAPI supports both traditional Dash operations (sending transactions, querying blocks) and Dash Platform (Evolution) operations (state transitions, data contracts).

Read more: https://docs.dash.org/projects/platform/en/stable/docs/explanations/dapi.html

## Architecture

DAPI consists of two main processes:

1. **API Process** (`api.js`) - Handles the main gRPC and JSON-RPC endpoints
2. **Core Streams Process** (`core-streams.js`) - Handles streaming endpoints like block headers and transactions

Each process serves different types of endpoints:

### API Process Endpoints
- Core endpoints for interacting with Dash Core
- Platform endpoints for Dash Platform (Evolution) services
- Legacy JSON-RPC endpoints

### Core Streams Process Endpoints
- Block headers streaming
- Transaction streaming with bloom filters
- Masternode list streaming

For more details about the DAPI architecture, see the [Architecture](./architecture.md) document.

## DAPI Interfaces

DAPI offers two main interfaces:

1. **gRPC** - Modern, high-performance interface using Protocol Buffers
2. **JSON-RPC** - Legacy interface that will be removed in the future.

The gRPC interface is recommended for new applications due to its better performance, type safety, and streaming capabilities.

## Getting Started

To connect to DAPI, you can use either:

- Dash SDK
  - https://docs.dash.org/projects/platform/en/stable/docs/sdk-js/overview.html
  - https://docs.dash.org/projects/platform/en/stable/docs/sdk-rs/overview.html
- Low-level DAPI Client
  - https://docs.dash.org/projects/platform/en/stable/docs/dapi-client-js/overview.html
- Generated gRPC clients 
  - https://github.com/dashpay/platform/tree/master/packages/dapi-grpc

For a step-by-step guide on setting up and using DAPI, see the [Getting Started](./getting-started.md) document.

## Dependencies

DAPI relies on several underlying services:

- **Dash Core** - Provides access to the Dash blockchain
- **Drive ABCI** - Provides access to the Dash Platform state
- **Tenderdash** - Consensus engine for Dash Platform

See the [Configuration](./configuration.md) document for details on how to set up these dependencies.
