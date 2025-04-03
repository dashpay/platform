# Dashmate Documentation

Welcome to the Dashmate documentation. This documentation provides a comprehensive overview of the Dashmate architecture, its components, and how they work together.

## Contents

- [Services](./services.md) - Overview of all dashmate services and their responsibilities
- [Networking](./networking.md) - Detailed explanation of how services communicate with each other

## Introduction to Dash Platform

Dash Platform is a technology stack for building decentralized applications on the Dash network. It provides developers with functionality for storing and retrieving application data through a decentralized API.

The platform is built as a Layer 2 solution on top of the Dash blockchain, leveraging the security and decentralization of the underlying network while providing the flexibility and ease of use needed for application development.

## Key Components

The platform consists of several key components:

1. **Core** - The Dash blockchain node that handles Layer 1 operations
2. **Drive** - The data storage and processing layer for application data
3. **DAPI** - The decentralized API that provides access to the platform

## Getting Started

To run your own Dash Platform node, refer to the main [README.md](../README.md) for setup instructions.

For developers looking to build applications on Dash Platform, check out the [Dash Platform SDK documentation](https://dashplatform.readme.io/).

## Architecture Overview

```
                                 ┌─────────────────┐
                                 │                 │
                                 │    Dashmate     │
                                 │    CLI & Helper │
                                 │                 │
                                 └─────────────────┘
                                         │
                                         │ manages
                                         ▼
┌──────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  ┌─────────────┐                 ┌───────────┐                       │
│  │             │                 │  Clients  │                       │
│  │   Core      │◄──────┐         └─────┬─────┘                       │
│  │  (Dash      │       │               │                             │
│  │  Blockchain)│       │               │ HTTP/gRPC                   │
│  │             │       │               ▼                             │
│  └─────────────┘       │       ┌───────────────┐                     │
│                        │       │               │                     │
│                        │       │    Gateway    │◄────────────────────┼── User HTTP
│                ┌───────┴───────┤               │                     │   Requests
│                │               └───────┬───────┘                     │
│                │              Platform │Layer                        │
│                │                       │                             │
│                │  ┌────────┐  ┌────────┐  ┌────────┐                │
│                │  │        │  │        │  │        │                │
│                │  │ Drive  │  │ Tender │  │ DAPI   │                │
│                │  │ ABCI   │  │ dash   │  │ API/   │                │
│                │  │        │  │        │  │ Streams│                │
│                │  └────────┘  └────────┘  └────────┘                │
│                │                                                     │
│                └─────────────────────────────────────────────────────┘
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

This architecture provides a robust and scalable platform for decentralized applications, with clear separation of concerns between different components.
