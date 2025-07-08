# subscribeToBlockHeadersWithChainLocks

## Client API

Streams block headers and chain locks.

**Request Parameters**:
- `fromBlockHash` or `fromBlockHeight`: Starting point (optional)
- `count`: Number of blocks to fetch (0 means subscribe to new blocks)

**Response Parameters**:
- Stream of block headers and chain locks
  - `rawBlockHeader`: Serialized block header
  - `rawChainLock`: Serialized chain lock

**Example Usage**:
```javascript
const stream = dapiClient.core.subscribeToBlockHeadersWithChainLocks({ fromBlockHeight: 1000, count: 10 });

stream.on('data', (response) => {
  if (response.rawBlockHeader) {
    console.log('Received block header');
  } else if (response.rawChainLock) {
    console.log('Received chain lock');
  }
});
```

## Internal Implementation

The `subscribeToBlockHeadersWithChainLocks` endpoint is implemented in the `subscribeToBlockHeadersWithChainLocksHandlerFactory.js` file.

### Implementation Details

1. **Dual Operation Modes**
   - **Historical Mode**: Retrieves a specific count of block headers from a starting point
   - **Streaming Mode**: Provides real-time updates when new blocks are created
   - **Combined Mode**: Historical headers followed by real-time streaming

2. **Historical Data Retrieval**
   - Uses an iterator pattern for efficient header retrieval
   - Supports two ways to specify the starting point:
     - `fromBlockHash`: Start from a specific block hash
     - `fromBlockHeight`: Start from a specific block height
   - Fetches headers in batches for better performance
   - Includes associated chain locks when available

3. **Real-time Streaming**
   - Uses a `ProcessMediator` for event-based communication
   - Subscribes to ZMQ events from Dash Core
   - Emits block headers and chain locks as they are received
   - Implements proper cleanup on client disconnection

4. **Flow Control**
   - Implements back-pressure handling
   - Uses acknowledgment for stream responses
   - Prevents overwhelming clients with data

5. **Dependencies**
   - BlockHeadersCache for efficient access to historical headers
   - ZMQ client for real-time notifications
   - ProcessMediator for event management

### Code Flow

```
Client Request 
  → gRPC Server 
    → subscribeToBlockHeadersWithChainLocksHandler 
      → Validate Parameters
      → If Historical Data Requested (count > 0):
        → Create Historical Headers Iterator
        → Stream Headers Until Count Reached
      → If Streaming Requested (count = 0 or all historical sent):
        → Subscribe to ZMQ Events
        → On New Block:
          → Format Block Header
          → Send to Client
        → On New ChainLock:
          → Format ChainLock
          → Send to Client
      → On Client Disconnect:
        → Clean Up Subscriptions
```

### Stream Lifecycle Management

The handler implements careful resource management:

1. **Initialization**
   - Sets up event listeners and iterators
   - Prepares caches for efficient data retrieval

2. **Active Streaming**
   - Monitors Core node for new blocks and chain locks
   - Transforms raw data to client-friendly format
   - Sends data through the gRPC stream

3. **Cleanup**
   - Detects client disconnection (explicit or timeout)
   - Removes event listeners and subscriptions
   - Releases resources to prevent memory leaks

This lifecycle management ensures efficient operation even with long-lived streaming connections.