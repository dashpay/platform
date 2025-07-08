# getBestBlockHeight

## Client API

Returns the height of the latest block in the chain.

**Request Parameters**: None

**Response Parameters**:
- `height`: Height of the best block (number)

**Example Usage**:
```javascript
const height = await dapiClient.core.getBestBlockHeight();
console.log(`Current block height: ${height}`);
```

## Internal Implementation

The `getBestBlockHeight` endpoint is implemented in the `getBestBlockHeightHandlerFactory.js` file.

### Implementation Details

1. **Caching Mechanism**
   - The implementation uses a caching strategy to avoid making frequent RPC calls to the Core node
   - The cache is invalidated when a new block is detected via ZMQ notifications
   - RPC calls to Core are only made when the cache is empty or invalid

2. **Handler Flow**
   - When a request is received, the handler first checks if a valid cached value exists
   - If cache is valid, returns the cached value immediately
   - If cache is invalid or empty, makes an RPC call to Core's `getBlockCount` method
   - The returned value is cached before being sent back to the client

3. **Dependencies**
   - Dash Core RPC interface
   - ZMQ for new block notifications (for cache invalidation)

4. **Error Handling**
   - Forwards any errors from the Core RPC call with appropriate gRPC error codes
   - Handles network connectivity issues with the Core node

### Code Flow

```
Client Request 
  → gRPC Server 
    → getBestBlockHeightHandler 
      → Check Cache
        → If Valid: Return Cached Height
        → If Invalid: Call Core RPC getBlockCount 
          → Cache Result
          → Return Height
```