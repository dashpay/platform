# getBlockchainStatus

## Client API

Returns status information about the Dash blockchain.

**Request Parameters**: None

**Response Parameters**:
- `chain`: Chain name (e.g., "main", "test", "regtest")
- `blocks`: Current block count
- `headers`: Current header count
- `bestBlockHash`: Hash of the best block
- `difficulty`: Current network difficulty
- `verificationProgress`: Sync progress (0-1)
- `chainWork`: Total chain work in hex
- `softForks`: Information about active and pending soft forks
- `protocolVersion`: P2P protocol version
- `version`: Core software version

**Example Usage**:
```javascript
const status = await dapiClient.core.getBlockchainStatus();
console.log(`Network: ${status.chain}, Blocks: ${status.blocks}`);
```

## Internal Implementation

The `getBlockchainStatus` endpoint is implemented in the `getBlockchainStatusHandlerFactory.js` file.

### Implementation Details

1. **Data Collection**
   - The implementation fetches data from two Core RPC calls in parallel:
     - `getBlockchainInfo`: For blockchain-specific information
     - `getNetworkInfo`: For network-specific details
   - Combines the results into a comprehensive response

2. **Caching Strategy**
   - Uses caching to reduce load on Core node
   - Cache is invalidated when new blocks are detected via ZMQ notifications

3. **Status Determination**
   - Calculates sync status (SYNCING or READY) based on verification progress value
   - Provides chain status details from Core's internal state

4. **Dependencies**
   - Dash Core RPC interface
   - ZMQ for new block notifications (for cache invalidation)

5. **Error Handling**
   - Maps Core RPC errors to appropriate gRPC error codes
   - Handles connection issues with informative error messages

### Code Flow

```
Client Request 
  → gRPC Server 
    → getBlockchainStatusHandler 
      → Check Cache
        → If Valid: Return Cached Status
        → If Invalid: 
          → Parallel Requests:
            → Call getBlockchainInfo RPC
            → Call getNetworkInfo RPC
          → Combine Results
          → Determine Sync Status
          → Cache Response
          → Return Status
```

### Response Construction

The handler combines information from multiple sources:
- Basic blockchain info: chain name, blocks, headers, best hash
- Network details: protocol version, services, connections
- Mining information: difficulty, chain work
- Software version information
- Soft fork activation statuses

This provides clients with a complete overview of the current blockchain state in a single call.