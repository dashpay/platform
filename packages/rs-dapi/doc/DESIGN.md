# rs-dapi Technical Design Document

## Overview

rs-dapi is a Rust implementation of the Dash Decentralized API (DAPI) that serves as a drop-in replacement for the existing JavaScript DAPI implementation. It provides gRPC and JSON-RPC endpoints for accessing both Dash Core (Layer 1) and Dash Platform (Layer 2) functionality through the masternode network.

rs-dapi operates behind Envoy as a reverse proxy gateway, which handles SSL termination, external security, protocol translation, and request routing. This architecture allows rs-dapi to focus on business logic while Envoy manages all external security concerns.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      External Clients                      │
│           (Web browsers, mobile apps, CLI tools)           │
└─────────────────────────┬───────────────────────────────────┘
                          │ HTTPS/WSS/gRPC-Web
                          │ (SSL termination & security)
┌─────────────────────────┼───────────────────────────────────┐
│                         │                                   │
│                    Envoy Gateway                            │
│                 (Managed by Dashmate)                      │
│                                                             │
│  • SSL/TLS termination        • Load balancing             │
│  • Protocol translation       • Rate limiting              │
│  • Authentication/authorization • Request routing          │
│  • CORS handling              • Health checking            │
└─────────────────────────┬───────────────────────────────────┘
                          │ HTTP/gRPC/WebSocket
                          │ (Internal network, trusted)
┌─────────────────────────┼───────────────────────────────────┐
│                         │                                   │
│                    rs-dapi                                  │
│              (Single Binary Process)                       │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                                                     │   │
│  │              Unified Server                         │   │
│  │                                                     │   │
│  │  ┌─────────────┐  ┌─────────────────────────────┐   │   │
│  │  │             │  │                             │   │   │
│  │  │ API Handler │  │ Streams Handler             │   │   │
│  │  │             │  │                             │   │   │
│  │  │ - Core gRPC │  │ - Block streaming           │   │   │
│  │  │ - Platform  │  │ - TX streaming              │   │   │
│  │  │ - JSON-RPC  │  │ - Masternode list streaming │   │   │
│  │  │             │  │                             │   │   │
│  │  └─────────────┘  └─────────────────────────────┘   │   │
│  │                                                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                  │
└─────────────────────────┼──────────────────────────────────┘
                          │
    ┌─────────────────────┼─────────────────┐
    │                     │                 │
    │          External Services            │
    │                                       │
    │  ┌──────────┐  ┌──────────────┐       │
    │  │   Dash   │  │ Tenderdash/  │       │
    │  │   Core   │  │    Drive     │       │
    │  │          │  │              │       │
    │  │ RPC+ZMQ  │  │ gRPC+WS+RPC  │       │
    │  └──────────┘  └──────────────┘       │
    └───────────────────────────────────────┘
```

## Core Components

### 1. Project Structure

```
packages/rs-dapi/
├── Cargo.toml
├── src/
│   ├── main.rs                    # Entry point and server initialization
│   ├── lib.rs                     # Library exports
│   ├── server.rs                  # Unified server implementation
│   ├── config/                    # Configuration management
│   │   ├── mod.rs
│   │   └── settings.rs
│   ├── protocol/                   # Protocol translation layer
│   │   ├── mod.rs
│   │   ├── grpc_native.rs         # Native gRPC protocol handler
│   │   ├── rest_translator.rs     # REST to gRPC translation
│   │   └── jsonrpc_translator.rs  # JSON-RPC to gRPC translation
│   ├── services/                  # gRPC service implementations (protocol-agnostic)
│   │   ├── mod.rs
│   │   ├── core_service.rs        # Core blockchain endpoints
│   │   ├── platform_service.rs    # Platform endpoints (main service implementation)
│   │   ├── platform_service/      # Modular complex method implementations
│   │   │   └── get_status.rs      # Complex get_status implementation with status building
│   │   └── streams_service.rs     # Streaming endpoints
│   ├── health/                    # Health and monitoring endpoints
│   │   ├── mod.rs
│   │   ├── status.rs             # Service status reporting
│   │   └── metrics.rs            # Prometheus metrics
│   ├── clients/                   # External API clients
│   │   ├── mod.rs
│   │   ├── dashcore.rs           # Dash Core RPC + ZMQ
│   │   ├── drive.rs              # Drive gRPC client
│   │   └── tenderdash.rs         # Tenderdash RPC + WebSocket
│   ├── handlers/                  # Business logic handlers (protocol-agnostic)
│   │   ├── mod.rs
│   │   ├── core_handlers.rs       # Core endpoint logic
│   │   ├── platform_handlers.rs   # Platform endpoint logic
│   │   └── stream_handlers.rs     # Streaming logic
│   ├── utils/                     # Shared utilities
│   │   ├── mod.rs
│   │   ├── validation.rs          # Input validation
│   │   ├── hash.rs               # Hash utilities
│   │   └── bloom_filter.rs       # Bloom filter implementation
│   ├── errors/                    # Error types and handling
│   │   ├── mod.rs
│   │   └── grpc_errors.rs        # gRPC error mapping
│   └── jsonrpc/                   # JSON-RPC server (deprecated, uses translation layer)
│       ├── mod.rs
│       └── server.rs
├── proto/                         # Generated protobuf code (if needed)
├── tests/                         # Integration tests
└── doc/                           # Documentation
    └── DESIGN.md                  # This document
```

### 2. Modular Service Architecture

rs-dapi implements a modular service architecture that separates simple proxy operations from complex business logic:

#### Architecture Principles
- **Separation of Concerns**: Complex methods are isolated in dedicated modules
- **Context Sharing**: All modules have access to service context without boilerplate
- **Maintainability**: Each complex operation lives in its own file for easy maintenance
- **Scalability**: New complex methods can be added as separate modules
- **No Macros**: Uses simple `impl` blocks instead of macro-generated code

#### Service Organization Pattern
```
services/
├── service_name.rs               # Main service implementation
│   ├── Service struct definition
│   ├── Simple proxy methods (majority of methods)
│   ├── Service initialization
│   └── Delegation calls to complex modules
├── service_name/                 # Directory for complex methods
│   ├── complex_method_1.rs       # First complex method implementation
│   ├── complex_method_2.rs       # Second complex method implementation
│   └── ...                       # Additional complex methods
└── shared_utilities.rs           # Shared helper modules
```

#### Implementation Pattern
Each complex method follows this pattern:

```rust
// Main service file (e.g., platform_service.rs)
mod complex_method;  // Import the complex implementation

impl GrpcTrait for ServiceImpl {
    async fn simple_method(&self, req: Request<Req>) -> Result<Response<Res>, Status> {
        // Simple proxy - direct forwarding
        match self.client.simple_method(req.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Client error: {}", e))),
        }
    }

    async fn complex_method(&self, req: Request<Req>) -> Result<Response<Res>, Status> {
        // Delegate to complex implementation
        self.complex_method_impl(req).await
    }
}

// Complex method file (e.g., service_name/complex_method.rs)
impl ServiceImpl {
    pub async fn complex_method_impl(&self, req: Request<Req>) -> Result<Response<Res>, Status> {
        // Full access to service context:
        // - self.clients (drive_client, tenderdash_client, etc.)
        // - self.cache
        // - self.config
        // Complex business logic here...
    }
}
```

#### Benefits
- **Clean Separation**: Simple methods stay in main file, complex logic isolated
- **Full Context Access**: Complex methods have access to all service state
- **Easy Testing**: Each complex method can be tested independently
- **Code Navigation**: Developers can quickly find specific functionality
- **Reduced File Size**: Main service files remain manageable
- **Parallel Development**: Different developers can work on different complex methods

### 3. External Dependencies

The implementation leverages existing Dash Platform crates and external libraries:

#### Platform Crates
- `dapi-grpc` - gRPC service definitions and generated code
- `rs-dpp` - Data Platform Protocol types and validation
- `rs-drive` - Drive client and proof operations

#### External Libraries
- `tokio` - Async runtime
- `tonic` - gRPC framework
- `tonic-web` - gRPC-Web support for browsers
- `tower` - Service framework and middleware
- `tower-http` - HTTP middleware and services
- `axum` - Modern HTTP framework for REST API
- `serde` - Serialization/deserialization
- `jsonrpc-core` + `jsonrpc-http-server` - JSON-RPC server
- `config` - Configuration management
- `tracing` - Structured logging
- `anyhow` + `thiserror` - Error handling
- `zmq` - ZeroMQ client for Dash Core
- `reqwest` - HTTP client for Tenderdash RPC
- `tokio-tungstenite` - WebSocket client for Tenderdash
- `prometheus` - Metrics collection
- `hyper` - HTTP implementation

## Service Implementations

### 4. Core Service

Implements blockchain-related gRPC endpoints (protocol-agnostic via translation layer):

#### Endpoints
- `getBlockchainStatus` - Network and chain status information
- `getBestBlockHeight` - Current blockchain height
- `getTransaction` - Transaction lookup by hash
- `broadcastTransaction` - Submit transactions to network

#### Key Features
- Direct integration with Dash Core RPC
- ZMQ notifications for real-time updates
- Transaction validation and error handling
- Network status aggregation
- **Protocol-Agnostic**: Works identically for gRPC, REST, and JSON-RPC clients

### 5. Platform Service

Implements Dash Platform gRPC endpoints (protocol-agnostic via translation layer) with a modular architecture for complex method implementations:

#### Modular Architecture
The Platform Service uses a modular structure where complex methods are separated into dedicated modules:

```
services/
├── platform_service.rs          # Main service implementation
│   ├── Struct definition (PlatformServiceImpl)
│   ├── Simple proxy methods (most Platform trait methods)
│   ├── Service initialization and configuration
│   └── Delegation to complex method modules
├── platform_service/            # Complex method implementations
│   └── get_status.rs            # Complex get_status implementation with integrated status building
```

#### Main Service (`platform_service.rs`)
- **Service Definition**: Contains `PlatformServiceImpl` struct with all necessary context
- **Simple Methods**: Direct proxy methods that forward requests to Drive client
- **Complex Method Delegation**: Delegates complex operations to specialized modules
- **Shared Context**: All struct fields marked `pub(crate)` for submodule access

#### Complex Method Modules (`platform_service/`)
- **Dedicated Files**: Each complex method gets its own module file
- **Context Access**: Full access to service context via `impl PlatformServiceImpl` blocks
- **Business Logic**: Contains all complex caching, validation, and processing logic
- **Integrated Utilities**: Status building and other utilities included directly in method modules
- **Clean Separation**: Isolated complex logic from simple proxy operations


#### Endpoints
- `broadcastStateTransition` - Submit state transitions
- `waitForStateTransitionResult` - Wait for processing with proof generation
- `getConsensusParams` - Platform consensus parameters
- `getStatus` - Platform status information
- Unimplemented endpoints (proxy to Drive ABCI)

#### Key Features
- **Modular Organization**: Complex methods separated into dedicated modules for maintainability
- **Context Sharing**: Submodules have full access to service context (clients, cache, config)
- **No Boilerplate**: Uses `impl` blocks rather than wrapper structs
- **Integrated Utilities**: Status building and other helper functions co-located with their usage
- State transition hash validation (64-character SHA256 hex)
- Integration with Drive for proof generation
- Tenderdash WebSocket monitoring for real-time events
- Timeout handling for long-running operations
- Error conversion from Drive responses
- **Protocol-Agnostic**: Identical behavior across all client protocols

### 6. Streams Service

Implements real-time streaming gRPC endpoints (protocol-agnostic via translation layer):

#### Endpoints
- `subscribeToBlockHeadersWithChainLocks` - Block header streaming
- `subscribeToTransactionsWithProofs` - Transaction filtering with bloom filters
- `subscribeToMasternodeList` - Masternode list updates

#### Key Features
- ZMQ event processing for real-time data
- Bloom filter management for transaction filtering
- Merkle proof generation for SPV verification
- Stream lifecycle management
- Connection resilience and reconnection
- **Protocol-Agnostic**: Streaming works consistently across all protocols

### 7. JSON-RPC Service (Legacy)

Provides legacy HTTP endpoints for backward compatibility via protocol translation:

#### Endpoints
- `getBestBlockHash` - Hash of the latest block
- `getBlockHash` - Block hash by height

#### Key Features
- **Translation Layer**: All requests converted to gRPC calls internally
- HTTP server with JSON-RPC 2.0 compliance
- Error format compatibility with existing clients
- Minimal subset focused on essential operations
- **Deprecated**: New clients should use gRPC or REST APIs

### 8. REST API Gateway

Provides RESTful HTTP endpoints via protocol translation layer:

#### Features
- **Protocol Translation**: Automatic REST to gRPC translation
- **OpenAPI Documentation**: Auto-generated API documentation
- **HTTP/JSON**: Standard REST patterns with JSON payloads
- **CORS Support**: Cross-origin resource sharing for web applications
- **Unified Backend**: All REST calls converted to gRPC internally

#### Example Endpoints
```
GET  /v1/core/blockchain-status     -> getBlockchainStatus
GET  /v1/core/best-block-height     -> getBestBlockHeight
GET  /v1/core/transaction/{hash}    -> getTransaction
POST /v1/core/broadcast-transaction -> broadcastTransaction

POST /v1/platform/broadcast-state-transition -> broadcastStateTransition
GET  /v1/platform/consensus-params            -> getConsensusParams
GET  /v1/platform/status                      -> getStatus
```

### 9. Health and Monitoring Endpoints

Built-in observability and monitoring capabilities:

#### Health Check Endpoints
- `GET /health` - Basic health status
- `GET /health/ready` - Readiness probe (all dependencies available)
- `GET /health/live` - Liveness probe (service is running)

#### Metrics Endpoints
- `GET /metrics` - Prometheus metrics
- `GET /metrics/json` - JSON format metrics

#### Status Information
- Service uptime and version
- External service connection status
- Request counts and latency statistics
- Error rates and types
- Active stream subscriber counts

## Data Flow and Processing

### 10. Multi-Protocol Server Architecture

rs-dapi implements a unified server with a protocol translation layer that normalizes all incoming requests to gRPC format, operating behind Envoy as a trusted backend service:

#### Protocol Translation Architecture
- **Protocol Translation Layer**: All non-gRPC protocols translated to gRPC format
- **Unified Business Logic**: All handlers work exclusively with gRPC messages
- **Single Code Path**: No protocol-specific logic in business layer
- **Native gRPC**: Direct pass-through for gRPC requests
- **Trusted Environment**: Operates in internal network behind Envoy gateway

#### Request Flow with Protocol Translation
```
External Client → Envoy Gateway → Protocol Translation → gRPC Services → External Services
      ↓              ↓                    ↓                  ↓               ↓
   HTTPS/WSS    SSL termination    ┌─────────────────┐   Core Service   Dash Core
   gRPC-Web  →  Protocol xlat   →  │ REST→gRPC xlat  │→  Platform Svc →  Drive    
   REST API     Rate limiting      │ JSON→gRPC xlat  │   Streams Svc    Tenderdash
                Auth/CORS          │ Native gRPC     │   
                                   └─────────────────┘   
                                   Protocol Translation Layer
```

#### Internal Architecture with Translation Layer
```
┌─────────────────────────────────────────────────────────────┐
│               rs-dapi Process (localhost only)             │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Protocol Translation Layer             │   │
│  │                                                     │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │   │
│  │  │    REST     │ │  JSON-RPC   │ │    gRPC     │   │   │
│  │  │ Translator  │ │ Translator  │ │   Native    │   │   │
│  │  │             │ │             │ │             │   │   │
│  │  │ HTTP→gRPC   │ │ JSON→gRPC   │ │ Pass-through│   │   │
│  │  └─────────────┘ └─────────────┘ └─────────────┘   │   │
│  │              │              │              │       │   │
│  │              └──────────────┼──────────────┘       │   │
│  │                             │                      │   │
│  │                             ▼                      │   │
│  │  ┌─────────────────────────────────────────────┐   │   │
│  │  │           gRPC Services Layer               │   │   │
│  │  │         (Protocol-Agnostic)                 │   │   │
│  │  │                                             │   │   │
│  │  │ ┌─────────────┐ ┌─────────────────────────┐ │   │   │
│  │  │ │ Core Service│ │   Platform & Streams    │ │   │   │
│  │  │ │             │ │       Services          │ │   │   │
│  │  │ │ - Blockchain│ │ - State transitions     │ │   │   │
│  │  │ │ - TX broadcast │ - Block streaming     │ │   │   │
│  │  │ │ - Status    │ │ - Masternode updates   │ │   │   │
│  │  │ └─────────────┘ └─────────────────────────┘ │   │   │
│  │  └─────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### Protocol Translation Details
- **REST Translator**: Converts HTTP/JSON requests to gRPC messages, handles OpenAPI compliance
- **JSON-RPC Translator**: Converts JSON-RPC 2.0 format to corresponding gRPC calls
- **gRPC Native**: Direct pass-through for native gRPC requests (no translation)
- **Response Translation**: Converts gRPC responses back to original protocol format
- **Error Translation**: Maps gRPC status codes to appropriate protocol-specific errors
- **Streaming**: gRPC streaming for real-time data, WebSocket support for REST

### 11. Protocol Translation Layer

The protocol translation layer is the key architectural component that enables unified business logic while supporting multiple client protocols:

#### Translation Components

##### REST to gRPC Translator
- **HTTP Method Mapping**: GET/POST/PUT/DELETE mapped to appropriate gRPC methods
- **Path Parameter Extraction**: REST path parameters converted to gRPC message fields
- **JSON Body Conversion**: HTTP JSON payloads converted to protobuf messages
- **Query Parameter Handling**: URL query parameters mapped to gRPC request fields
- **Response Translation**: gRPC responses converted back to JSON with proper HTTP status codes
- **Error Mapping**: gRPC status codes mapped to appropriate HTTP status codes

##### JSON-RPC to gRPC Translator  
- **RPC Method Mapping**: JSON-RPC method names mapped to gRPC service methods
- **Parameter Conversion**: JSON-RPC params converted to gRPC message fields
- **ID Tracking**: JSON-RPC request IDs preserved for response correlation
- **Batch Request Support**: Multiple JSON-RPC requests in single batch handled
- **Error Format**: gRPC errors converted to JSON-RPC 2.0 error format

##### Native gRPC Handler
- **Direct Pass-through**: No translation required for native gRPC requests
- **Metadata Preservation**: gRPC metadata and headers preserved
- **Streaming Support**: Full bidirectional streaming support
- **Compression**: Native gRPC compression and optimization

#### Translation Examples

##### REST to gRPC Translation Example
```
# REST Request
GET /v1/core/transaction/abc123def456
Accept: application/json

# Translated to gRPC
service: CoreService
method: getTransaction  
message: GetTransactionRequest {
  hash: "abc123def456"
}

# gRPC Response translated back to REST
HTTP/1.1 200 OK
Content-Type: application/json
{
  "transaction": { ... },
  "blockHash": "...",
  "confirmations": 42
}
```

##### JSON-RPC to gRPC Translation Example
```
# JSON-RPC Request
{
  "jsonrpc": "2.0",
  "method": "getBestBlockHeight", 
  "id": 1
}

# Translated to gRPC
service: CoreService
method: getBestBlockHeight
message: GetBestBlockHeightRequest {}

# gRPC Response translated back to JSON-RPC
{
  "jsonrpc": "2.0",
  "result": {
    "height": 850000
  },
  "id": 1
}
```

#### Benefits of Translation Layer Architecture
- **Single Business Logic**: All protocols use the same underlying gRPC services
- **Consistent Behavior**: Identical business logic regardless of client protocol
- **Easy Testing**: Only need to test gRPC services, translations are simpler
- **Maintainability**: Changes to business logic automatically apply to all protocols
- **Performance**: Minimal translation overhead, native gRPC performance
- **Type Safety**: Strong typing from protobuf definitions enforced across all protocols

### 11. State Transition Processing

The `waitForStateTransitionResult` endpoint follows this flow:

1. **Input Validation**
   - Check Tenderdash connection availability
   - Validate state transition hash format (64-char hex)
   - Parse request parameters (hash, prove flag)

2. **Transaction Monitoring**
   - Wait for transaction to be included in a block
   - Monitor Tenderdash events via WebSocket
   - Handle timeout scenarios with appropriate errors

3. **Proof Generation** (if requested)
   - Fetch proof from Drive for the state transition
   - Include metadata and proof data in response

4. **Error Handling**
   - Convert Drive errors to gRPC status codes
   - Handle timeout with `DEADLINE_EXCEEDED`
   - Map transaction errors to structured responses

### 12. Streaming Data Processing

#### Transaction Filtering
1. Client subscribes with bloom filter
2. ZMQ notifications from Dash Core processed
3. Transactions tested against bloom filters
4. Matching transactions sent with merkle proofs

#### Block Header Streaming
1. ZMQ block notifications from Dash Core
2. Block headers extracted and validated
3. Chain lock information included
4. Streamed to subscribed clients

### 13. External Service Integration

#### Dash Core Integration
- **RPC Client**: Blockchain queries, transaction broadcasting
- **ZMQ Client**: Real-time notifications (blocks, transactions, chainlocks)
- **Connection Management**: Retry logic, health checks

#### Drive Integration
- **gRPC Client**: State queries, proof generation
- **Error Mapping**: Drive-specific errors to gRPC status codes
- **Connection Pooling**: Efficient resource utilization

#### Tenderdash Integration
- **RPC Client**: Consensus queries, network status
- **WebSocket Client**: Real-time Platform events
- **Event Processing**: State transition monitoring

## Configuration and Deployment

### 14. Configuration Management

#### Environment Variables
- `DAPI_GRPC_SERVER_PORT` - gRPC API server port (default: 3005, internal)
- `DAPI_GRPC_STREAMS_PORT` - gRPC streams server port (default: 3006, internal)  
- `DAPI_JSON_RPC_PORT` - JSON-RPC server port (default: 3004, internal)
- `DAPI_REST_GATEWAY_PORT` - REST API gateway port (default: 8080, internal)
- `DAPI_HEALTH_CHECK_PORT` - Health and metrics port (default: 9090, internal)
- `DAPI_BIND_ADDRESS` - Bind address for all services (default: 127.0.0.1, internal only)
- `DAPI_NETWORK` - Network selection (mainnet/testnet/devnet)
- `DAPI_LIVENET` - Production mode flag
- `DAPI_ENABLE_REST` - Enable REST API gateway (default: false)
- Dash Core connection settings (RPC + ZMQ)
- Drive connection settings (gRPC)
- Tenderdash connection settings (RPC + WebSocket)

#### Process Architecture
- **Single Binary**: One process handles all DAPI functionality behind Envoy
- **Multi-threaded**: Tokio runtime with multiple worker threads
- **Shared State**: Common configuration and client connections
- **Service Isolation**: Logical separation of Core, Platform, and Streams services
- **Internal Network**: All services bind to localhost/internal addresses only
- **Trusted Backend**: No direct external exposure, operates behind Envoy gateway

#### Configuration Files
- TOML-based configuration with environment override
- Network-specific default configurations
- Validation and error reporting for invalid configs

### 15. Binary Architecture

The rs-dapi binary is designed as a unified server that handles all DAPI functionality:

#### Single Process Design
- **Unified Server**: Single process serving all endpoints
- **Multiple gRPC Services**: Core, Platform, and Streams services on different ports
- **Integrated JSON-RPC**: HTTP server embedded within the same process
- **Shared Resources**: Common connection pools and state management

#### Port Configuration (Internal Network Only)
- **gRPC API Port** (default: 3005): Core + Platform endpoints (localhost binding)
- **gRPC Streams Port** (default: 3006): Streaming endpoints (localhost binding)
- **JSON-RPC Port** (default: 3004): Legacy HTTP endpoints (localhost binding)
- **REST Gateway Port** (default: 8080): REST API for gRPC services (localhost binding)
- **Health/Metrics Port** (default: 9090): Monitoring endpoints (localhost binding)

All ports bind to internal addresses only (127.0.0.1). External access is handled by Envoy.
- **Health/Metrics Port** (default: 9090): Monitoring and status endpoints

#### Service Startup
```bash
# Single command starts all services and dependencies
rs-dapi

# Optional configuration override
rs-dapi --config /path/to/config.toml

# Development mode with verbose logging
rs-dapi --log-level debug
```

#### Multi-Protocol Support
- **gRPC Services**: Core and Platform endpoints on port 3005, Streams on port 3006
- **JSON-RPC Server**: Legacy HTTP endpoints on port 3004
- **REST API**: Optional REST gateway for gRPC services (configurable port)
- **Health/Monitoring Endpoints**: Built-in status and metrics endpoints

#### Protocol Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                        External Network                    │
│              (Internet clients, HTTPS/WSS/gRPC-Web)        │
└─────────────────────────┬───────────────────────────────────┘
                          │ SSL/TLS encrypted
┌─────────────────────────┼───────────────────────────────────┐
│                    Envoy Gateway                           │
│  • SSL termination     • Protocol translation             │
│  • Rate limiting        • Load balancing                   │
│  • CORS/Auth           • Health checking                   │
└─────────────────────────┬───────────────────────────────────┘
                          │ Internal HTTP/gRPC (unencrypted)
┌─────────────────────────┼───────────────────────────────────┐
│               rs-dapi Process (localhost only)             │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Protocol Translation Layer             │   │
│  │                                                     │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │   │
│  │  │    REST     │ │  JSON-RPC   │ │    gRPC     │   │   │
│  │  │:8080 (HTTP) │ │:3004 (HTTP) │ │:3005/:3006  │   │   │
│  │  │             │ │             │ │             │   │   │
│  │  │ HTTP→gRPC   │ │ JSON→gRPC   │ │ Pass-through│   │   │
│  │  │ Translator  │ │ Translator  │ │   Native    │   │   │
│  │  └─────────────┘ └─────────────┘ └─────────────┘   │   │
│  │          │               │               │         │   │
│  │          └───────────────┼───────────────┘         │   │
│  │                          ▼                         │   │
│  │  ┌─────────────────────────────────────────────┐   │   │
│  │  │           gRPC Services Layer               │   │   │
│  │  │         (Protocol-Agnostic)                 │   │   │
│  │  │                                             │   │   │
│  │  │ ┌─────────────┐ ┌─────────────────────────┐ │   │   │
│  │  │ │ Core Service│ │   Platform & Streams    │ │   │   │
│  │  │ │             │ │       Services          │ │   │   │
│  │  │ │ - Blockchain│ │ - State transitions     │ │   │   │
│  │  │ │ - TX broadcast │ - Block streaming     │ │   │   │
│  │  │ │ - Status    │ │ - Masternode updates   │ │   │   │
│  │  │ └─────────────┘ └─────────────────────────┘ │   │   │
│  │  └─────────────────────────────────────────────┘   │   │
│  │                                                     │   │
│  │  ┌─────────────────────────────────────────────┐   │   │
│  │  │      Health/Metrics :9090 (localhost)      │   │   │
│  │  └─────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### Dashmate Integration
- **Drop-in Replacement**: Direct substitution for JavaScript DAPI processes
- **Same Configuration**: Uses existing environment variables and setup
- **Compatible Deployment**: Works with current dashmate deployment scripts
- **Envoy Gateway**: Integrates with existing Envoy configuration in dashmate
- **Internal Service**: Operates as trusted backend behind Envoy proxy
- **Resource Efficiency**: Single process reduces memory footprint and complexity
- **Automatic Startup**: All services and dependencies start with single command
- **Built-in Monitoring**: Health endpoints accessible to Envoy for health checks

### 16. Error Handling Strategy

#### gRPC Error Mapping
- `INVALID_ARGUMENT` - Input validation failures
- `UNAVAILABLE` - External service connectivity issues
- `DEADLINE_EXCEEDED` - Operation timeouts
- `INTERNAL` - Unexpected internal errors
- `NOT_FOUND` - Resource not found

#### Error Context
- Structured error messages with context
- Request correlation IDs for tracing
- Detailed error metadata for debugging
- Compatible error formats with JavaScript DAPI

## Performance and Scalability

### 17. Performance Characteristics

#### Async Processing
- Tokio runtime with work-stealing scheduler
- Non-blocking I/O for all external communications
- Concurrent request handling

#### Resource Management
- Connection pooling for external services
- Efficient memory usage with zero-copy operations
- Stream backpressure handling

#### Caching Strategy
- Blockchain status caching with TTL
- Connection keep-alive for external services
- Smart invalidation based on ZMQ events

### 18. Monitoring and Observability

#### Logging
- Structured logging with `tracing`
- Request/response logging with correlation IDs
- Performance metrics and timing information
- Protocol-specific logging (gRPC, REST, JSON-RPC)

#### Built-in Metrics
- **Request Metrics**: Counts, latency histograms per protocol
- **Connection Metrics**: External service status and health
- **Stream Metrics**: Active subscribers, message throughput
- **System Metrics**: Memory usage, CPU utilization, goroutine counts
- **Business Metrics**: Transaction success rates, proof generation times

#### Prometheus Integration
- Native Prometheus metrics endpoint
- Custom metrics for DAPI-specific operations
- Grafana-compatible dashboards
- Alerting rules for operational monitoring

#### Health Checks
- Service readiness and liveness endpoints
- External service connectivity validation
- Graceful degradation strategies

## Security Considerations

### 19. Envoy Gateway Security Model

rs-dapi operates in a trusted environment behind Envoy Gateway, which handles all external security concerns:

#### External Security (Handled by Envoy)
- **SSL/TLS Termination**: All external HTTPS/WSS connections terminated at Envoy
- **Certificate Management**: SSL certificates managed by dashmate/Envoy configuration
- **Rate Limiting**: Request rate limiting and DDoS protection at gateway level
- **CORS Handling**: Cross-origin resource sharing policies enforced by Envoy
- **Authentication/Authorization**: Client authentication and authorization at gateway
- **Protocol Translation**: Secure gRPC-Web, WebSocket, and HTTPS to internal HTTP/gRPC

#### Internal Security (rs-dapi Responsibility)
- **Input Validation**: SHA256 hash format validation, buffer overflow prevention
- **Request Sanitization**: Input sanitization for all endpoints and parameters
- **Request Size Limits**: Maximum request size enforcement
- **Connection Limits**: Maximum concurrent connections per internal service
- **Trust Boundary**: Only accepts connections from localhost/internal network

### 20. Network Architecture Security

#### Trust Model
- **Trusted Internal Network**: rs-dapi assumes all requests come from trusted Envoy
- **No Direct External Exposure**: All services bind to localhost (127.0.0.1) only
- **Network Isolation**: External network access only through Envoy gateway
- **Service Mesh**: Can be integrated with service mesh for additional internal security

#### Internal Communication Security
- **Dash Core Integration**: Secure RPC connections with authentication credentials
- **Drive Integration**: Internal gRPC connections within trusted network
- **Tenderdash Integration**: Authenticated RPC and WebSocket connections
- **Credential Management**: Secure storage and rotation of service credentials

## Testing Strategy

### 21. Test Coverage

#### Unit Tests
- Individual component testing
- Mock external services
- Error condition testing
- Input validation testing

#### Integration Tests
- End-to-end service testing
- External service integration
- Stream lifecycle testing
- Error propagation testing

#### Performance Tests
- Load testing under various conditions
- Memory usage profiling
- Connection limit testing
- Concurrent client testing

## Migration and Compatibility

### 22. Compatibility Requirements

#### API Compatibility
- Identical gRPC service definitions
- Same JSON-RPC endpoint behavior
- Compatible error response formats
- Matching timeout behaviors

#### Configuration Compatibility
- Same environment variable names
- Compatible configuration file formats
- Identical default values
- Same network selection logic

### 23. Deployment Strategy

#### Gradual Migration
1. **Dashmate Integration**: Update dashmate to use rs-dapi binary
2. **Feature Flag Deployment**: Deploy with feature flags for rollback capability
3. **Traffic Validation**: Monitor performance and error rates in production
4. **Full Migration**: Complete replacement of JavaScript DAPI once validated

#### Deployment in Dashmate
- **Binary Replacement**: rs-dapi replaces existing JavaScript DAPI processes
- **Envoy Integration**: Works seamlessly with existing Envoy gateway configuration
- **Configuration Compatibility**: Uses same environment variables as current setup
- **Internal Network Binding**: All services bind to localhost, external access via Envoy
- **Process Management**: Single process simplifies service management in dashmate
- **Resource Optimization**: Reduced memory usage and inter-process communication overhead
- **Security Simplification**: No SSL/certificate management needed in rs-dapi

#### Rollback Strategy
- Feature flags for easy rollback
- Traffic routing controls
- Monitoring and alerting
- Automated rollback triggers

## Future Considerations

### 24. Extensibility

#### Plugin Architecture
- Modular service design
- Configurable middleware
- Extension points for custom logic

#### Performance Optimizations
- Custom allocators for high-frequency operations
- SIMD optimizations for cryptographic operations
- Advanced caching strategies

### 25. Maintenance and Updates

#### Code Organization
- Clear module boundaries
- Comprehensive documentation
- Automated testing and CI/CD
- Regular dependency updates

#### Monitoring and Debugging
- Advanced debugging capabilities
- Performance profiling tools
- Memory leak detection
- Crash reporting and analysis

---

This design document serves as the foundation for implementing rs-dapi and will be updated as the implementation progresses and requirements evolve.
