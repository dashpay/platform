# getBestBlockHash

## Client API

Returns the hash of the best (tip) block in the longest blockchain.

**Request Format**:
```json
{
  "jsonrpc": "2.0",
  "method": "getBestBlockHash",
  "params": [],
  "id": 1
}
```

**Response Format**:
```json
{
  "jsonrpc": "2.0",
  "result": "000000000000001bb82a7f5973618cfd3588ba1df2ee3004d4add6321678564b",
  "id": 1
}
```

**Example Usage**:
```javascript
// Using fetch API
const response = await fetch('http://localhost:2501', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'getBestBlockHash',
    params: [],
    id: 1
  })
});
const data = await response.json();
console.log(`Best block hash: ${data.result}`);
```

## Internal Implementation

The `getBestBlockHash` endpoint is implemented in the `getBestBlockHash.js` file.

### Implementation Details

1. **Caching Mechanism**
   - Implements a caching strategy to avoid repeated calls to Core
   - Cache is invalidated when new blocks are detected via ZMQ
   - Only makes a Core RPC call when the cache is empty or invalid

2. **Core Integration**
   - Interfaces with Dash Core via RPC to get the latest block hash
   - Uses the Core's `getBestBlockHash` method directly

3. **Event Handling**
   - Subscribes to ZMQ `hashblock` events from Core
   - Uses these events to detect when new blocks are added to the chain
   - Invalidates the cache when a new block is detected

4. **Error Handling**
   - Wraps Core RPC errors in JSON-RPC compatible error responses
   - Maintains consistency with JSON-RPC 2.0 specification

5. **Dependencies**
   - Dash Core RPC interface
   - ZMQ client for new block notifications

### Code Flow

```
Client Request 
  → JSON-RPC Server 
    → getBestBlockHash Handler 
      → Check Cache
        → If Valid: Return Cached Hash
        → If Invalid: 
          → Call Core RPC getBestBlockHash
          → Cache Result
          → Return Hash
```

### Endpoint Categorization

This endpoint is categorized as a Layer 1 (L1) endpoint in the API documentation, indicating that it interacts directly with the Dash Core blockchain functionality.