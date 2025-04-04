# Dashmate services

This document provides an overview of all dashmate services, their responsibilities, and how they communicate with each other.
The platform is designed as a collection of microservices that work together to provide a complete blockchain-based application platform.

## Overview

Dashmate runs and orchestrate Dash Platform components:

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
│  ┌─────────────┐                                                     │
│  │             │                                                     │
│  │   Core      │◄──────┐                                             │
│  │  (Dash      │       │                                             │
│  │  Blockchain)│       │                                             │
│  │             │       │                                             │
│  └─────────────┘       │                                             │
│                        │                                             │
│                        │                                             │
│                ┌───────┴──────────────────────────────────┐          │
│                │                                          │          │
│                │              Platform Layer              │          │
│                │                                          │          │
│                │  ┌────────┐  ┌────────┐  ┌────────┐      │          │
│                │  │        │  │        │  │        │      │          │
│                │  │ Drive  │  │ Tender │  │ DAPI   │      │          │
│                │  │ ABCI   │  │ dash   │  │ API/   │      │          │
│                │  │        │  │        │  │ Streams│      │          │
│                │  └────────┘  └────────┘  └────────┘      │          │
│                │                                          │          │
│                └──────────────────────────────────────────┘          │
│                                   ▲                                  │
│                                   │                                  │
│                          ┌────────────────┐                          │
│                          │                │                          │
│                          │    Gateway     │◄─────────────────────────┼── User HTTP
│                          │                │                          │   Requests
│                          └────────────────┘                          │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

* The Gateway handles external HTTP\HTTPS requests, provides TLS termination, and routes traffic to the correct service.
* Dash Core is the Dash blockchain node, handling Layer 1 operations (consensus, masternodes, payment transactions).
* Platform is layer 2 that provide platform functionality:
  * Drive ABCI is runtime for Platform layer 2 chain. Data contracts, and identities logic is implemented here.
  * Tenderdash is the consensus engine for the platform, providing BFT consensus for state transitions.
  * DAPI (decentralized API) exposes the platform functionalities via gRPC, and streaming interfaces, serving as the main interface for client developers.
* Dashmate orchestrates the services, providing a CLI and helper service for managing the platform.

## Services

- [Core](./core.md): Dash Core service
- [Platform](./platform.md): Platform services (Drive, Tenderdash, DAPI)
- [Gateway](./gateway.md): API Gateway service
- [Dashmate Helper](./dashmate_helper.md): Helper service for Dashmate CLI

## Port Security Considerations

### Summary of Exposed Ports

| Type                | Ports                                                        | Default Host Binding |
|---------------------|--------------------------------------------------------------|---------------------|
| **Public-facing**   | Core P2P (9999)<br>Tenderdash P2P (26656)<br>Gateway API (443) | 0.0.0.0 (all)       |
| **Localhost-only**  | Core RPC (9998)<br>Insight UI (3001)<br>Dashmate Helper (9100)<br>Drive ABCI Metrics (29090)<br>Drive Debug Tools (6669, 8083)<br>Tenderdash RPC (26657)<br>Tenderdash Metrics (26660)<br>Tenderdash Debug (6060)<br>Gateway Metrics (9090)<br>Gateway Admin (9901)<br>Rate Limiter Metrics (9102) | 127.0.0.1 (local)   |
| **Internal only**   | Core ZMQ (29998)<br>Drive ABCI (26658)<br>Drive gRPC (26670)<br>DAPI JSON-RPC (3004)<br>DAPI gRPC (3005)<br>DAPI Streams (3006)<br>Rate Limiter gRPC (8081)<br>Rate Limiter StatsD (9125)<br>Rate Limiter Redis (6379) | (not exposed)       |

### Port Security Notes

- **Public-facing ports** (0.0.0.0): Only ports that need to be accessible from other machines bind to all interfaces by default.
- **Localhost-only ports** (127.0.0.1): Most ports bind only to localhost by default for security.
- **Internal-only ports**: These ports are not bound to any host interface and are only accessible from other containers in the Docker network.
- **RPC Security**: Core RPC has an explicit allowlist in the configuration: `core.rpc.allowIps` (default: ['127.0.0.1', '172.16.0.0/12', '192.168.0.0/16'])

## Network Configuration

The default Docker network configuration can be customized with the following settings:

- Network Subnet: `docker.network.subnet` (default: "0.0.0.0/0")
- Container addresses are assigned automatically by Docker within the subnet range

The platform uses different Docker Compose profiles to enable specific functionality:
- `core`: Core and optional Insight services
- `platform`: All platform-specific services

## Data Persistence

Several volumes are used for data persistence:
- `core_data`: Stores blockchain data
- `drive_abci_data`: Stores platform state data
- `drive_tenderdash`: Stores consensus data

## Metrics and Monitoring

Most services provide metrics endpoints:
- Drive ABCI: Platform application metrics
- Tenderdash: Consensus metrics
- Gateway: API gateway metrics
- Rate Limiter: Rate limiting metrics

These can be integrated with monitoring systems like Prometheus and Grafana.
