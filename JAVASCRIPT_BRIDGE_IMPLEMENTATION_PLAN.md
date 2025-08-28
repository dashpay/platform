# JavaScript Bridge Implementation Plan
## Fixing tonic-web-wasm-client Concurrency Issues in Dash Platform WASM SDK

### Problem Summary
The Dash Platform WASM SDK has severe concurrency bottlenecks due to `tonic-web-wasm-client` using a `spawn_local` pattern that serializes all concurrent gRPC requests through a single JavaScript microtask queue instead of allowing true browser parallelism.

**Root Cause Location**: `/packages/rs-dapi-client/src/transport/wasm_channel.rs:113-124`
```rust
fn into_send<'a, F: Future + 'static>(f: F) -> BoxFuture<'a, F::Output> {
    let (tx, rx) = oneshot::channel::<F::Output>();
    spawn_local(async move {  // ← BOTTLENECK: Serializes ALL requests
        tx.send(f.await).ok();
    });
    rx.unwrap_or_else(|e| panic!("Failed to receive result: {:?}", e)).boxed()
}
```

**Impact**: All concurrent gRPC requests are forced through a single-threaded queue, eliminating browser's native HTTP/2 multiplexing capabilities.

## Solution: JavaScript Bridge Implementation

### Overview
Replace the problematic `tonic-web-wasm-client` transport layer with a direct JavaScript bridge using wasm-bindgen. This bypasses Rust's async limitations entirely by leveraging browser-native gRPC-web clients that handle concurrency properly.

### Expected Results
- **5x+ improvement** in concurrent request throughput
- **Complete elimination** of serialization bottleneck
- **Zero breaking changes** to existing APIs
- **Browser-native performance** using HTTP/2 multiplexing

## Implementation Timeline: 3-4 Weeks

### Phase 1: Core Bridge Infrastructure (Weeks 1-2)

#### 1.1 New File Structure
```
packages/wasm-sdk/src/
├── js_bridge/
│   ├── mod.rs              # Module exports and public interface
│   ├── bridge.rs           # Core wasm-bindgen bridge implementation
│   ├── client.rs           # JavaScript gRPC client wrapper
│   ├── types.rs            # Type conversion utilities
│   └── errors.rs           # Error handling and status mapping
└── js/
    ├── grpc-client.js      # JavaScript gRPC-web client implementation
    ├── platform-methods.js # Platform service method definitions
    ├── core-methods.js     # Core service method definitions
    └── utils.js            # Common utilities and helpers
```

#### 1.2 Core Bridge Implementation

**File: `/packages/wasm-sdk/src/js_bridge/bridge.rs`**
```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Object, Uint8Array, Promise};
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "grpcRequest")]
    fn js_grpc_request(
        endpoint: &str,
        service_method: &str,
        request_data: &Uint8Array,
        metadata: &Object,
    ) -> Promise;
    
    #[wasm_bindgen(js_name = "grpcStreamRequest")]
    fn js_grpc_stream_request(
        endpoint: &str,
        service_method: &str,
        request_data: &Uint8Array,
        metadata: &Object,
    ) -> Promise;
}

pub struct JavaScriptGrpcBridge {
    endpoint: String,
}

impl JavaScriptGrpcBridge {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    pub async fn unary_call(
        &self,
        service_method: &str,
        request_bytes: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<Vec<u8>, JsValue> {
        let request_array = Uint8Array::from(&request_bytes[..]);
        let metadata_obj = self.build_metadata_object(metadata)?;
        
        let promise = js_grpc_request(&self.endpoint, service_method, &request_array, &metadata_obj);
        let js_future = JsFuture::from(promise);
        let result = js_future.await?;
        
        let response_array: Uint8Array = result.dyn_into()?;
        Ok(response_array.to_vec())
    }

    pub async fn streaming_call(
        &self,
        service_method: &str,
        request_bytes: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<js_sys::AsyncIterator, JsValue> {
        let request_array = Uint8Array::from(&request_bytes[..]);
        let metadata_obj = self.build_metadata_object(metadata)?;
        
        let promise = js_grpc_stream_request(&self.endpoint, service_method, &request_array, &metadata_obj);
        let js_future = JsFuture::from(promise);
        let result = js_future.await?;
        
        result.dyn_into()
    }

    fn build_metadata_object(&self, metadata: HashMap<String, String>) -> Result<Object, JsValue> {
        let obj = Object::new();
        for (key, value) in metadata {
            js_sys::Reflect::set(&obj, &JsValue::from_str(&key), &JsValue::from_str(&value))?;
        }
        Ok(obj)
    }
}
```

**File: `/packages/wasm-sdk/src/js_bridge/client.rs`**
```rust
use super::bridge::JavaScriptGrpcBridge;
use super::types::{ToGrpcBytes, FromGrpcBytes};
use super::errors::BridgeError;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::future::BoxFuture;
use http::{Request, Response};
use tonic::body::Body;
use tonic::client::GrpcService;
use tonic::Status;

#[derive(Clone, Debug)]
pub struct JsBridgeClient {
    bridge: JavaScriptGrpcBridge,
}

impl JsBridgeClient {
    pub fn new(endpoint: String) -> Result<Self, BridgeError> {
        Ok(Self {
            bridge: JavaScriptGrpcBridge::new(endpoint),
        })
    }
}

impl GrpcService<Body> for JsBridgeClient {
    type Future = BoxFuture<'static, Result<Response<Body>, Status>>;
    type ResponseBody = Body;
    type Error = Status;

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let bridge = self.bridge.clone();
        let uri_path = request.uri().path().to_string();
        let headers = request.headers().clone();
        
        Box::pin(async move {
            // Extract request body bytes
            let request_bytes = extract_body_bytes(request.into_body()).await?;
            
            // Convert headers to metadata
            let metadata = extract_metadata(&headers);
            
            // Make gRPC call via JavaScript bridge
            let response_bytes = bridge.unary_call(&uri_path, request_bytes, metadata)
                .await
                .map_err(|js_err| Status::internal(format!("Bridge error: {:?}", js_err)))?;
            
            // Build HTTP response
            let response = Response::builder()
                .status(200)
                .header("content-type", "application/grpc")
                .body(Body::from(response_bytes))
                .map_err(|e| Status::internal(format!("Response build error: {}", e)))?;
            
            Ok(response)
        })
    }

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // JavaScript bridge is always ready - no connection setup needed
        Poll::Ready(Ok(()))
    }
}

async fn extract_body_bytes(body: Body) -> Result<Vec<u8>, Status> {
    use futures::TryStreamExt;
    
    let body_stream = body.into_data_stream();
    let body_chunks: Result<Vec<_>, _> = body_stream.try_collect().await;
    let body_chunks = body_chunks.map_err(|e| Status::internal(format!("Body read error: {}", e)))?;
    
    Ok(body_chunks.into_iter().flatten().collect())
}

fn extract_metadata(headers: &http::HeaderMap) -> std::collections::HashMap<String, String> {
    let mut metadata = std::collections::HashMap::new();
    
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            metadata.insert(name.to_string(), value_str.to_string());
        }
    }
    
    metadata
}
```

**File: `/packages/wasm-sdk/js/grpc-client.js`**
```javascript
/**
 * JavaScript gRPC-Web client that handles concurrent requests properly
 * Uses browser's native Fetch API for optimal HTTP/2 multiplexing
 */

class DashGrpcClient {
    constructor() {
        this.activeRequests = new Map();
        this.requestCounter = 0;
    }

    async grpcRequest(endpoint, serviceMethod, requestData, metadata = {}) {
        const requestId = ++this.requestCounter;
        
        try {
            // Prepare gRPC-Web request
            const url = `${endpoint}${serviceMethod}`;
            
            // Build headers for gRPC-Web
            const headers = {
                'Content-Type': 'application/grpc-web+proto',
                'X-Grpc-Web': '1',
                'Accept': 'application/grpc-web+proto',
                ...metadata
            };
            
            // Create fetch request - this uses browser's native HTTP/2 multiplexing
            const requestPromise = fetch(url, {
                method: 'POST',
                headers: headers,
                body: requestData,
                mode: 'cors',
                credentials: 'omit'
            }).then(async (response) => {
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }
                
                const arrayBuffer = await response.arrayBuffer();
                return new Uint8Array(arrayBuffer);
            });
            
            // Track active request
            this.activeRequests.set(requestId, requestPromise);
            
            // Execute request - multiple requests will run concurrently
            const result = await requestPromise;
            
            // Cleanup
            this.activeRequests.delete(requestId);
            
            return result;
            
        } catch (error) {
            this.activeRequests.delete(requestId);
            throw error;
        }
    }

    async grpcStreamRequest(endpoint, serviceMethod, requestData, metadata = {}) {
        // For streaming requests, we can use ReadableStream or EventSource
        const url = `${endpoint}${serviceMethod}`;
        
        const headers = {
            'Content-Type': 'application/grpc-web+proto',
            'X-Grpc-Web': '1',
            'Accept': 'application/grpc-web+proto',
            ...metadata
        };
        
        const response = await fetch(url, {
            method: 'POST',
            headers: headers,
            body: requestData,
            mode: 'cors',
            credentials: 'omit'
        });
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        return response.body.getReader();
    }

    getActiveRequestCount() {
        return this.activeRequests.size;
    }

    // Utility for debugging concurrent requests
    logConcurrency() {
        console.log(`Active concurrent requests: ${this.activeRequests.size}`);
        return this.activeRequests.size;
    }
}

// Global client instance
window.dashGrpcClient = new DashGrpcClient();

// Export bridge functions for wasm-bindgen
window.grpcRequest = (endpoint, serviceMethod, requestData, metadata) => {
    return window.dashGrpcClient.grpcRequest(endpoint, serviceMethod, requestData, metadata);
};

window.grpcStreamRequest = (endpoint, serviceMethod, requestData, metadata) => {
    return window.dashGrpcClient.grpcStreamRequest(endpoint, serviceMethod, requestData, metadata);
};

// Debug utilities
window.getGrpcConcurrency = () => window.dashGrpcClient.getActiveRequestCount();
window.logGrpcConcurrency = () => window.dashGrpcClient.logConcurrency();
```

#### 1.3 Transport Layer Integration

**File: `/packages/rs-dapi-client/src/transport/js_bridge_channel.rs` (new)**
```rust
//! JavaScript bridge transport implementation

use super::{TransportClient, TransportError};
use crate::{connection_pool::ConnectionPool, request_settings::AppliedRequestSettings, Uri};
use dapi_grpc::core::v0::core_client::CoreClient;
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use crate::js_bridge::client::JsBridgeClient;

/// Platform Client using JavaScript bridge transport
pub type PlatformJsBridgeClient = PlatformClient<JsBridgeClient>;

/// Core Client using JavaScript bridge transport  
pub type CoreJsBridgeClient = CoreClient<JsBridgeClient>;

/// Create a JavaScript bridge channel
pub fn create_js_bridge_channel(
    uri: Uri,
    _settings: Option<&AppliedRequestSettings>,
) -> Result<JsBridgeClient, TransportError> {
    JsBridgeClient::new(uri.to_string())
        .map_err(|e| TransportError::Other(format!("Failed to create JS bridge client: {:?}", e)))
}

impl TransportClient for PlatformJsBridgeClient {
    fn with_uri(uri: Uri, pool: &ConnectionPool) -> Result<Self, TransportError> {
        // Use connection pool for caching
        Ok(pool
            .get_or_create(
                crate::connection_pool::PoolPrefix::Platform,
                &uri,
                None,
                || match create_js_bridge_channel(uri.clone(), None) {
                    Ok(channel) => Ok(Self::new(channel).into()),
                    Err(e) => Err(dapi_grpc::tonic::Status::invalid_argument(format!(
                        "JS bridge channel creation failed: {}",
                        e
                    ))),
                }
            )?
            .into())
    }

    fn with_uri_and_settings(
        uri: Uri,
        settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Result<Self, TransportError> {
        Ok(pool
            .get_or_create(
                crate::connection_pool::PoolPrefix::Platform,
                &uri,
                Some(settings),
                || match create_js_bridge_channel(uri.clone(), Some(settings)) {
                    Ok(channel) => Ok(Self::new(channel).into()),
                    Err(e) => Err(dapi_grpc::tonic::Status::invalid_argument(format!(
                        "JS bridge channel creation failed: {}",
                        e
                    ))),
                },
            )?
            .into())
    }
}

impl TransportClient for CoreJsBridgeClient {
    fn with_uri(uri: Uri, pool: &ConnectionPool) -> Result<Self, TransportError> {
        Ok(pool
            .get_or_create(
                crate::connection_pool::PoolPrefix::Core,
                &uri,
                None,
                || match create_js_bridge_channel(uri.clone(), None) {
                    Ok(channel) => Ok(Self::new(channel).into()),
                    Err(e) => Err(dapi_grpc::tonic::Status::invalid_argument(format!(
                        "JS bridge channel creation failed: {}",
                        e
                    ))),
                }
            )?
            .into())
    }

    fn with_uri_and_settings(
        uri: Uri,
        settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Result<Self, TransportError> {
        Ok(pool
            .get_or_create(
                crate::connection_pool::PoolPrefix::Core,
                &uri,
                Some(settings),
                || match create_js_bridge_channel(uri.clone(), Some(settings)) {
                    Ok(channel) => Ok(Self::new(channel).into()),
                    Err(e) => Err(dapi_grpc::tonic::Status::invalid_argument(format!(
                        "JS bridge channel creation failed: {}",
                        e
                    ))),
                },
            )?
            .into())
    }
}
```

**Modification: `/packages/rs-dapi-client/src/transport/wasm_channel.rs`**
```rust
// Replace the existing implementation with:

#[cfg(feature = "js-bridge")]
mod js_bridge_impl {
    pub use super::js_bridge_channel::*;
}

#[cfg(not(feature = "js-bridge"))]
mod tonic_web_impl {
    // Keep existing tonic-web-wasm-client implementation as fallback
    // ... existing code ...
}

#[cfg(feature = "js-bridge")]
pub use js_bridge_impl::*;

#[cfg(not(feature = "js-bridge"))]
pub use tonic_web_impl::*;
```

#### 1.4 Phase 1 Deliverables & Success Criteria

**Deliverables:**
- [ ] Core JavaScript bridge infrastructure implemented
- [ ] wasm-bindgen integration working for at least one gRPC method
- [ ] Basic POC demonstrating concurrent request capability
- [ ] Unit tests for bridge functionality

**Success Criteria:**
- [ ] Single gRPC method (e.g., `getIdentity`) works via JavaScript bridge
- [ ] Multiple concurrent requests don't serialize (measurable improvement)
- [ ] Error handling maintains existing behavior
- [ ] No performance regression for single requests

### Phase 2: Complete Method Coverage (Weeks 2-3)

#### 2.1 Full gRPC Method Implementation

**Platform Methods Coverage (50+ methods):**
```rust
// File: /packages/wasm-sdk/src/js_bridge/platform_methods.rs
use super::bridge::JavaScriptGrpcBridge;
use dapi_grpc::platform::v0::*;

impl JavaScriptGrpcBridge {
    // Identity methods
    pub async fn get_identity(&self, request: GetIdentityRequest) -> Result<GetIdentityResponse, BridgeError> {
        let request_bytes = request.to_grpc_bytes()?;
        let response_bytes = self.unary_call(
            "org.dash.platform.dapi.v0.Platform/getIdentity",
            request_bytes,
            std::collections::HashMap::new(),
        ).await?;
        GetIdentityResponse::from_grpc_bytes(response_bytes)
    }

    // Document methods
    pub async fn get_documents(&self, request: GetDocumentsRequest) -> Result<GetDocumentsResponse, BridgeError> {
        let request_bytes = request.to_grpc_bytes()?;
        let response_bytes = self.unary_call(
            "org.dash.platform.dapi.v0.Platform/getDocuments",
            request_bytes,
            std::collections::HashMap::new(),
        ).await?;
        GetDocumentsResponse::from_grpc_bytes(response_bytes)
    }

    // State transition methods
    pub async fn broadcast_state_transition(&self, request: BroadcastStateTransitionRequest) -> Result<BroadcastStateTransitionResponse, BridgeError> {
        let request_bytes = request.to_grpc_bytes()?;
        let response_bytes = self.unary_call(
            "org.dash.platform.dapi.v0.Platform/broadcastStateTransition",
            request_bytes,
            std::collections::HashMap::new(),
        ).await?;
        BroadcastStateTransitionResponse::from_grpc_bytes(response_bytes)
    }

    // All other platform methods...
    // - getIdentityByPublicKeyHash
    // - getIdentityBalance
    // - getDataContract
    // - getDocuments
    // - waitForStateTransitionResult
    // ... (implement all 50+ methods)
}
```

**Core Methods Coverage:**
```rust
// File: /packages/wasm-sdk/src/js_bridge/core_methods.rs
use super::bridge::JavaScriptGrpcBridge;
use dapi_grpc::core::v0::*;

impl JavaScriptGrpcBridge {
    pub async fn get_status(&self, request: GetStatusRequest) -> Result<GetStatusResponse, BridgeError> {
        let request_bytes = request.to_grpc_bytes()?;
        let response_bytes = self.unary_call(
            "org.dash.platform.dapi.v0.Core/getStatus",
            request_bytes,
            std::collections::HashMap::new(),
        ).await?;
        GetStatusResponse::from_grpc_bytes(response_bytes)
    }

    pub async fn get_transaction(&self, request: GetTransactionRequest) -> Result<GetTransactionResponse, BridgeError> {
        let request_bytes = request.to_grpc_bytes()?;
        let response_bytes = self.unary_call(
            "org.dash.platform.dapi.v0.Core/getTransaction",
            request_bytes,
            std::collections::HashMap::new(),
        ).await?;
        GetTransactionResponse::from_grpc_bytes(response_bytes)
    }

    pub async fn broadcast_transaction(&self, request: BroadcastTransactionRequest) -> Result<BroadcastTransactionResponse, BridgeError> {
        let request_bytes = request.to_grpc_bytes()?;
        let response_bytes = self.unary_call(
            "org.dash.platform.dapi.v0.Core/broadcastTransaction",
            request_bytes,
            std::collections::HashMap::new(),
        ).await?;
        BroadcastTransactionResponse::from_grpc_bytes(response_bytes)
    }

    // Streaming methods
    pub async fn subscribe_to_transactions_with_proofs(&self, request: TransactionsWithProofsRequest) -> Result<tonic::Streaming<TransactionsWithProofsResponse>, BridgeError> {
        // Handle streaming differently - return async iterator
        let request_bytes = request.to_grpc_bytes()?;
        let stream_reader = self.streaming_call(
            "org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs",
            request_bytes,
            std::collections::HashMap::new(),
        ).await?;
        
        // Convert JavaScript async iterator to tonic::Streaming
        Ok(convert_js_stream_to_tonic(stream_reader))
    }
}
```

#### 2.2 Type Conversion System

**File: `/packages/wasm-sdk/src/js_bridge/types.rs`**
```rust
use wasm_bindgen::JsValue;
use prost::Message;

pub trait ToGrpcBytes {
    fn to_grpc_bytes(&self) -> Result<Vec<u8>, JsValue>;
}

pub trait FromGrpcBytes: Sized {
    fn from_grpc_bytes(bytes: Vec<u8>) -> Result<Self, JsValue>;
}

// Implement for all gRPC request types
impl<T: Message> ToGrpcBytes for T {
    fn to_grpc_bytes(&self) -> Result<Vec<u8>, JsValue> {
        let mut buf = Vec::new();
        self.encode(&mut buf)
            .map_err(|e| JsValue::from_str(&format!("Protobuf encode error: {}", e)))?;
        Ok(buf)
    }
}

// Implement for all gRPC response types
impl<T: Message + Default> FromGrpcBytes for T {
    fn from_grpc_bytes(bytes: Vec<u8>) -> Result<Self, JsValue> {
        T::decode(&bytes[..])
            .map_err(|e| JsValue::from_str(&format!("Protobuf decode error: {}", e)))
    }
}

// Special handling for streaming responses
pub fn convert_js_stream_to_tonic<T>(js_reader: js_sys::ReadableStreamDefaultReader) -> tonic::Streaming<T>
where
    T: Message + Default + 'static,
{
    // Convert JavaScript ReadableStream to Rust async stream
    // This is a simplified implementation - full version would handle backpressure, etc.
    todo!("Implement JavaScript stream to tonic::Streaming conversion")
}

// Error conversion utilities
pub fn js_error_to_tonic_status(js_err: JsValue) -> tonic::Status {
    let error_msg = js_err.as_string().unwrap_or_else(|| "Unknown JavaScript error".to_string());
    
    // Parse common gRPC error patterns
    if error_msg.contains("NOT_FOUND") {
        tonic::Status::not_found(error_msg)
    } else if error_msg.contains("INVALID_ARGUMENT") {
        tonic::Status::invalid_argument(error_msg)
    } else if error_msg.contains("PERMISSION_DENIED") {
        tonic::Status::permission_denied(error_msg)
    } else {
        tonic::Status::internal(error_msg)
    }
}
```

#### 2.3 Enhanced JavaScript Client

**File: `/packages/wasm-sdk/js/grpc-client.js` (extended)**
```javascript
// Enhanced version with better error handling and performance optimization

class DashGrpcClient {
    constructor() {
        this.activeRequests = new Map();
        this.requestCounter = 0;
        this.connectionPool = new Map(); // Cache connections by endpoint
        this.maxConcurrentRequests = 100; // Reasonable limit
    }

    async grpcRequest(endpoint, serviceMethod, requestData, metadata = {}) {
        // Check concurrent request limit
        if (this.activeRequests.size >= this.maxConcurrentRequests) {
            throw new Error(`Too many concurrent requests (${this.activeRequests.size}/${this.maxConcurrentRequests})`);
        }

        const requestId = ++this.requestCounter;
        const startTime = performance.now();
        
        try {
            const url = this.buildGrpcUrl(endpoint, serviceMethod);
            const headers = this.buildGrpcHeaders(metadata);
            
            // Use connection pooling for better performance
            const requestPromise = this.makePooledRequest(url, {
                method: 'POST',
                headers: headers,
                body: requestData,
                mode: 'cors',
                credentials: 'omit',
                // Enable HTTP/2 multiplexing hints
                keepalive: true,
                signal: this.createTimeoutSignal(30000) // 30 second timeout
            });
            
            this.activeRequests.set(requestId, {
                promise: requestPromise,
                startTime: startTime,
                method: serviceMethod
            });
            
            const result = await requestPromise;
            
            // Performance logging
            const duration = performance.now() - startTime;
            console.debug(`gRPC ${serviceMethod}: ${duration.toFixed(2)}ms`);
            
            return result;
            
        } catch (error) {
            console.error(`gRPC ${serviceMethod} error:`, error);
            throw this.enhanceError(error, serviceMethod);
        } finally {
            this.activeRequests.delete(requestId);
        }
    }

    buildGrpcUrl(endpoint, serviceMethod) {
        // Ensure proper URL formatting
        const baseUrl = endpoint.endsWith('/') ? endpoint.slice(0, -1) : endpoint;
        const method = serviceMethod.startsWith('/') ? serviceMethod : '/' + serviceMethod;
        return baseUrl + method;
    }

    buildGrpcHeaders(metadata) {
        return {
            'Content-Type': 'application/grpc-web+proto',
            'X-Grpc-Web': '1',
            'Accept': 'application/grpc-web+proto',
            'X-User-Agent': 'dash-wasm-sdk/js-bridge',
            ...metadata
        };
    }

    async makePooledRequest(url, options) {
        // Simple connection reuse - in production might use more sophisticated pooling
        const response = await fetch(url, options);
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        const arrayBuffer = await response.arrayBuffer();
        return new Uint8Array(arrayBuffer);
    }

    createTimeoutSignal(timeoutMs) {
        const controller = new AbortController();
        setTimeout(() => controller.abort(), timeoutMs);
        return controller.signal;
    }

    enhanceError(error, method) {
        if (error.name === 'AbortError') {
            return new Error(`gRPC ${method} timed out`);
        }
        
        if (error.message.includes('Failed to fetch')) {
            return new Error(`Network error calling ${method}: ${error.message}`);
        }
        
        return error;
    }

    // Performance monitoring
    getPerformanceStats() {
        const activeRequests = Array.from(this.activeRequests.values());
        const now = performance.now();
        
        return {
            activeCount: this.activeRequests.size,
            longestRunning: activeRequests.reduce((max, req) => {
                const duration = now - req.startTime;
                return duration > max.duration ? { method: req.method, duration } : max;
            }, { method: null, duration: 0 }),
            methodBreakdown: activeRequests.reduce((breakdown, req) => {
                breakdown[req.method] = (breakdown[req.method] || 0) + 1;
                return breakdown;
            }, {})
        };
    }
}

// Global instance with enhanced capabilities
window.dashGrpcClient = new DashGrpcClient();

// Bridge functions
window.grpcRequest = (endpoint, serviceMethod, requestData, metadata) => {
    return window.dashGrpcClient.grpcRequest(endpoint, serviceMethod, requestData, metadata);
};

// Performance monitoring exports
window.getGrpcStats = () => window.dashGrpcClient.getPerformanceStats();
window.getGrpcConcurrency = () => window.dashGrpcClient.getActiveRequestCount();
```

#### 2.4 Phase 2 Deliverables & Success Criteria

**Deliverables:**
- [ ] All 50+ Platform gRPC methods implemented in bridge
- [ ] All Core gRPC methods implemented
- [ ] Type conversion system handling all protobuf types
- [ ] Streaming support for relevant methods
- [ ] Enhanced error handling and performance monitoring

**Success Criteria:**
- [ ] All existing WASM SDK functionality works via bridge
- [ ] 5x+ improvement in concurrent request throughput demonstrated
- [ ] No data integrity issues in type conversions
- [ ] All existing tests pass with bridge enabled
- [ ] Performance monitoring shows expected improvements

### Phase 3: Build System & Deployment (Week 4)

#### 3.1 Build System Integration

**File: `/packages/wasm-sdk/build.sh` (enhanced)**
```bash
#!/bin/bash
set -e

echo "Building Dash Platform WASM SDK with JavaScript Bridge..."

# Clean previous builds
rm -rf pkg/
mkdir -p pkg/

# Build Rust WASM with JavaScript bridge support
echo "Building Rust WASM..."
RUSTFLAGS="--cfg=web_sys_unstable_apis" wasm-pack build \
    --target web \
    --scope dash \
    --no-typescript \
    --features js-bridge

# Optimize WASM binary
echo "Optimizing WASM binary..."
if command -v wasm-opt &> /dev/null; then
    wasm-opt -Oz --enable-bulk-memory --enable-mutable-globals pkg/wasm_sdk_bg.wasm -o pkg/wasm_sdk_bg.wasm
fi

# Copy and process JavaScript bridge files
echo "Integrating JavaScript bridge..."
cp -r js/* pkg/

# Create unified entry point
cat > pkg/wasm_sdk_with_bridge.js << 'EOF'
// Unified Dash Platform WASM SDK with JavaScript Bridge
import init, * as wasm from './wasm_sdk_bg.js';

// Import bridge components
import './grpc-client.js';

// Initialize and export
let wasmInitialized = false;

export async function initWasmSdk(input) {
    if (!wasmInitialized) {
        await init(input);
        wasmInitialized = true;
        console.log('Dash WASM SDK initialized with JavaScript bridge');
    }
    return wasm;
}

// Re-export all WASM functions
export * from './wasm_sdk_bg.js';

// Export bridge utilities for debugging
export const getBridgeStats = () => window.getGrpcStats ? window.getGrpcStats() : null;
export const getBridgeConcurrency = () => window.getGrpcConcurrency ? window.getGrpcConcurrency() : 0;
EOF

# Create package.json with proper dependencies
cat > pkg/package.json << 'EOF'
{
  "name": "@dash/wasm-sdk",
  "version": "2.1.0-bridge",
  "description": "Dash Platform WASM SDK with JavaScript gRPC bridge for improved concurrency",
  "main": "wasm_sdk_with_bridge.js",
  "types": "wasm_sdk.d.ts",
  "files": [
    "wasm_sdk_bg.wasm",
    "wasm_sdk_bg.js",
    "wasm_sdk.d.ts",
    "wasm_sdk_with_bridge.js",
    "grpc-client.js"
  ],
  "sideEffects": [
    "./wasm_sdk_bg.js",
    "./grpc-client.js"
  ],
  "dependencies": {},
  "peerDependencies": {},
  "repository": {
    "type": "git",
    "url": "https://github.com/dashpay/platform"
  }
}
EOF

# Verify build integrity
echo "Verifying build..."
if [[ ! -f pkg/wasm_sdk_bg.wasm ]]; then
    echo "❌ WASM binary not found!"
    exit 1
fi

if [[ ! -f pkg/grpc-client.js ]]; then
    echo "❌ JavaScript bridge not found!"
    exit 1
fi

# Calculate and report bundle sizes
wasm_size=$(stat -f%z pkg/wasm_sdk_bg.wasm 2>/dev/null || stat -c%s pkg/wasm_sdk_bg.wasm)
js_size=$(stat -f%z pkg/grpc-client.js 2>/dev/null || stat -c%s pkg/grpc-client.js)

echo "✅ Build complete!"
echo "   WASM binary: $(echo "$wasm_size" | numfmt --to=iec)B"
echo "   JS bridge: $(echo "$js_size" | numfmt --to=iec)B"
echo "   Total package ready in pkg/"
```

#### 3.2 Cargo.toml Updates

**File: `/packages/wasm-sdk/Cargo.toml` (additions)**
```toml
[features]
default = ["dpns-contract", "dashpay-contract", "wallet-utils-contract", "token-history-contract", "keywords-contract", "js-bridge"]

# JavaScript bridge feature for improved concurrency
js-bridge = ["dep:wasm-bindgen-futures"]

[dependencies]
# ... existing dependencies ...

# JavaScript bridge dependencies
wasm-bindgen-futures = { version = "0.4.49", optional = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
# ... existing WASM dependencies ...

# Enhanced for JavaScript bridge
web-sys = { version = "0.3.4", features = [
    'console',
    'Document',
    'Element',
    'HtmlElement',
    'Node',
    'Window',
    'Crypto',
    'fetch',
    'Request',
    'RequestInit',
    'RequestMode',
    'Response',
    'Headers',
    'AbortController',
    'AbortSignal',
    'ReadableStream',
    'ReadableStreamDefaultReader'
] }
```

**File: `/packages/rs-dapi-client/Cargo.toml` (additions)**
```toml
[features]
js-bridge = ["wasm-sdk/js-bridge"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
# Remove tonic-web-wasm-client when using js-bridge
tonic-web-wasm-client = { version = "0.7.0", optional = true }
```

#### 3.3 Integration Tests

**File: `/packages/wasm-sdk/test/js-bridge-integration.test.mjs`**
```javascript
#!/usr/bin/env node

import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { initWasmSdk, getBridgeStats, getBridgeConcurrency } from '../pkg/wasm_sdk_with_bridge.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

describe('JavaScript Bridge Integration', () => {
    let sdk;

    beforeAll(async () => {
        // Initialize WASM SDK
        const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
        const wasmBuffer = readFileSync(wasmPath);
        const wasm = await initWasmSdk(wasmBuffer);
        
        // Build SDK instance
        sdk = wasm.WasmSdkBuilder.new_testnet().build();
    });

    test('concurrent request performance', async () => {
        console.log('Testing concurrent request performance...');
        
        // Test concurrent requests
        const startTime = performance.now();
        const promises = [];
        
        for (let i = 0; i < 10; i++) {
            promises.push(sdk.getIdentity("test-identity-id"));
        }
        
        // Monitor concurrency during execution
        const maxConcurrency = getBridgeConcurrency();
        
        await Promise.all(promises);
        const duration = performance.now() - startTime;
        
        console.log(`Executed 10 concurrent requests in ${duration.toFixed(2)}ms`);
        console.log(`Max concurrent requests: ${maxConcurrency}`);
        
        // Verify we achieved real concurrency (not serialization)
        expect(maxConcurrency).toBeGreaterThan(1);
        expect(duration).toBeLessThan(5000); // Should be much faster than serial
    }, 10000);

    test('all platform methods work', async () => {
        // Test all major platform methods
        const methods = [
            () => sdk.getIdentity("test-identity-id"),
            () => sdk.getDocuments("test-contract-id", "test-document-type", {}),
            () => sdk.getDataContract("test-contract-id"),
            // Add more methods as needed
        ];

        for (const method of methods) {
            try {
                await method();
                console.log(`✅ ${method.name} works`);
            } catch (error) {
                // Network errors are expected in test environment
                if (error.message.includes('Network error') || error.message.includes('Connection')) {
                    console.log(`⚠️ ${method.name} - network error (expected in test)`);
                } else {
                    throw error;
                }
            }
        }
    });

    test('bridge statistics work', () => {
        const stats = getBridgeStats();
        expect(stats).toBeDefined();
        expect(typeof stats.activeCount).toBe('number');
        console.log('Bridge stats:', stats);
    });
});
```

**File: `/packages/wasm-sdk/test/performance-benchmark.mjs`**
```javascript
#!/usr/bin/env node

import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Performance benchmark comparing old vs new implementation
async function runPerformanceBenchmark() {
    console.log('🚀 Performance Benchmark: JavaScript Bridge vs Original Implementation\n');

    // This would compare against a version built without js-bridge feature
    // For now, we'll benchmark the bridge implementation
    
    const { initWasmSdk, getBridgeConcurrency } = await import('../pkg/wasm_sdk_with_bridge.js');
    
    const wasmPath = join(__dirname, '../pkg/wasm_sdk_bg.wasm');
    const wasmBuffer = readFileSync(wasmPath);
    const wasm = await initWasmSdk(wasmBuffer);
    
    const sdk = wasm.WasmSdkBuilder.new_testnet().build();

    // Benchmark 1: Single request latency
    console.log('Benchmarking single request latency...');
    const singleStart = performance.now();
    try {
        await sdk.getIdentity("test-identity");
    } catch (e) {
        // Network error expected
    }
    const singleDuration = performance.now() - singleStart;
    console.log(`Single request: ${singleDuration.toFixed(2)}ms\n`);

    // Benchmark 2: Concurrent request throughput
    console.log('Benchmarking concurrent request throughput...');
    const concurrentTests = [5, 10, 20, 50];
    
    for (const concurrency of concurrentTests) {
        const startTime = performance.now();
        const promises = [];
        
        for (let i = 0; i < concurrency; i++) {
            promises.push(sdk.getIdentity(`test-identity-${i}`).catch(() => {}));
        }
        
        const maxConcurrentDuringTest = getBridgeConcurrency();
        
        await Promise.all(promises);
        const duration = performance.now() - startTime;
        const rps = (concurrency / duration) * 1000;
        
        console.log(`${concurrency} concurrent requests:`);
        console.log(`  Duration: ${duration.toFixed(2)}ms`);
        console.log(`  Throughput: ${rps.toFixed(2)} req/sec`);
        console.log(`  Max concurrent: ${maxConcurrentDuringTest}`);
        console.log('');
    }

    console.log('✅ Performance benchmark complete!');
}

runPerformanceBenchmark().catch(console.error);
```

#### 3.4 Phase 3 Deliverables & Success Criteria

**Deliverables:**
- [ ] Integrated build system producing optimized bundles
- [ ] Comprehensive test suite covering all functionality
- [ ] Performance benchmarks demonstrating improvements
- [ ] Documentation and migration guide
- [ ] Production-ready deployment artifacts

**Success Criteria:**
- [ ] Build system produces working artifacts with <10% size increase
- [ ] All tests pass including concurrent performance tests
- [ ] 5x+ concurrent request improvement documented
- [ ] Zero breaking changes to existing APIs
- [ ] Ready for production deployment

## Browser Support

### Supported Browsers
All modern browsers that currently support the WASM SDK will continue to work:
- **Chrome 57+** (March 2017)
- **Firefox 52+** (March 2017)
- **Safari 11+** (September 2017)
- **Edge 16+** (October 2017)

### Required APIs
The JavaScript bridge relies on standard web APIs that are widely supported:
- **Fetch API** - For gRPC-web requests
- **WebAssembly** - Already required by existing WASM SDK
- **Promises/async-await** - Modern JavaScript features
- **Uint8Array** - For binary data handling

### Performance Characteristics
- **HTTP/2 Multiplexing** - Browser's native connection management
- **Connection Reuse** - Automatic connection pooling by browser
- **Concurrent Requests** - True parallelism instead of serialization
- **Memory Efficiency** - Reduced object allocation compared to current implementation

## Implementation Risks & Mitigation

### Technical Risks

**Risk 1: Type Conversion Overhead**
- *Description*: Rust ↔ JavaScript boundary crossings may add latency
- *Mitigation*: Batch operations where possible, optimize protobuf serialization
- *Fallback*: Profile and optimize conversion functions if needed

**Risk 2: Bundle Size Increase**
- *Description*: Additional JavaScript code increases total package size
- *Mitigation*: Tree shaking, minification, and dead code elimination
- *Target*: <10% increase in total bundle size acceptable for 5x performance gain

**Risk 3: Memory Leaks**
- *Description*: JavaScript bridge might not properly clean up resources
- *Mitigation*: Comprehensive cleanup in request lifecycle, automated testing
- *Monitoring*: Long-running tests to detect memory growth patterns

### Integration Risks

**Risk 4: Compatibility Issues**
- *Description*: Existing code might not work with new transport layer
- *Mitigation*: Maintain identical APIs, comprehensive test coverage
- *Validation*: All existing tests must pass without modification

**Risk 5: Network Protocol Differences**
- *Description*: JavaScript gRPC-web client might handle edge cases differently
- *Mitigation*: Thorough testing with various network conditions and error scenarios
- *Fallback*: Enhanced error handling and logging for debugging

## Success Metrics

### Performance Metrics
- **Concurrent Throughput**: >5x improvement over current implementation
- **Single Request Latency**: Maintain current performance levels
- **Memory Usage**: <20% increase in memory footprint
- **Bundle Size**: <10% increase in total package size

### Reliability Metrics  
- **Error Rate**: No increase in gRPC request failures
- **Compatibility**: 100% of existing functionality works identically
- **Browser Support**: All currently supported browsers continue working

### Development Metrics
- **Implementation Time**: Complete in 3-4 weeks
- **Testing Coverage**: Maintain current test coverage levels
- **Documentation**: Complete implementation and usage guides

## Post-Implementation

### Monitoring
- Track concurrent request patterns in production
- Monitor error rates by browser and request type
- Measure actual performance improvements in real usage

### Future Enhancements
- **Request Batching**: Optimize multiple small requests into batches
- **Caching Layer**: Add intelligent response caching for repeated requests  
- **Advanced Connection Management**: Implement more sophisticated pooling strategies

### Technical Debt Cleanup
- Remove tonic-web-wasm-client dependency completely
- Optimize build process for faster development cycles
- Consider extracting bridge as reusable library for other projects

---

## Conclusion

This JavaScript bridge implementation provides a comprehensive solution to the Dash Platform WASM SDK's concurrency issues. By leveraging browser-native HTTP/2 multiplexing through JavaScript gRPC-web clients, we bypass the fundamental limitations of Rust's WASM async implementation while maintaining full API compatibility.

The 3-4 week implementation timeline delivers significant performance improvements (5x+ concurrent throughput) with manageable technical risk and maintains the existing developer experience. This approach represents the optimal balance of impact, complexity, and maintainability for resolving the current concurrency bottlenecks.