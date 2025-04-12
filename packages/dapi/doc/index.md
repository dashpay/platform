# DAPI Documentation

Welcome to the DAPI (Decentralized API) documentation. This guide provides comprehensive information about DAPI, its architecture, configuration, and available endpoints.

## What is DAPI?

DAPI (Decentralized API) is the decentralized HTTP API layer for the Dash Evolution platform. It provides a simple interface for accessing both the Dash blockchain (Core) and the Dash Platform (Evolution) features.

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

## Documentation Sections

### Getting Started
- [Getting Started](./getting-started.md) - Quick start guide for developers

### Core Documentation
- [Architecture](./architecture.md) - Detailed explanation of DAPI's design
- [Configuration](./configuration.md) - How to configure DAPI for different environments

### Endpoints
- [Endpoints Overview](./endpoints/index.md) - Index of all available API endpoints
- [Core Endpoints](./endpoints/core-endpoints.md) - Interact with the Dash blockchain
- [Platform Endpoints](./endpoints/platform-endpoints.md) - Interact with Dash Platform
- [JSON-RPC Endpoints](./endpoints/json-rpc-endpoints.md) - Legacy endpoints

## Client Libraries

Rather than directly interacting with DAPI's gRPC or JSON-RPC interfaces, most developers should use one of the official client libraries:

- [Dash SDK (JavaScript)](https://docs.dash.org/projects/platform/en/stable/docs/sdk-js/overview.html)
- [Dash SDK (Rust)](https://docs.dash.org/projects/platform/en/stable/docs/sdk-rs/overview.html) 
- [DAPI Client (JavaScript)](https://docs.dash.org/projects/platform/en/stable/docs/dapi-client-js/overview.html)
- [gRPC Clients](https://github.com/dashpay/platform/tree/master/packages/dapi-grpc) (for other languages)

These libraries handle the complexity of interacting with DAPI and provide idiomatic interfaces in their respective languages.