# getStatus

## Client API

Retrieves the current status of the Dash Platform.

**Request Parameters**: None

**Response Parameters**:
- `version`: Version information
  - `protocolVersion`: Protocol version number
  - `software`: Software version string
- `time`: Time information
  - `blockTime`: Last block time
  - `genesisTime`: Blockchain genesis time
  - `localTime`: Local node time
- `chain`: Chain information
  - `blockHeight`: Current block height
  - `blocksPerDay`: Average blocks per day
  - `bestBlockHash`: Hash of the best block
  - `syncStatus`: Sync status (SYNCING or READY)
- `sync`: Sync information
  - `startingBlockHeight`: Height when sync started
  - `currentBlockHeight`: Current sync height
  - `latestHeight`: Latest known height in the network
- `network`: Network information
  - `peerCount`: Number of connected peers
  - `isListening`: Whether node is listening for connections
  - `chainId`: Chain identifier string
- `nodeId`: Node identifier
- `proTxHash`: Protx hash (for masternodes)

**Example Usage**:
```javascript
const status = await dapiClient.platform.getStatus();
console.log(`Platform block height: ${status.chain.blockHeight}`);
console.log(`Platform version: ${status.version.software}`);
console.log(`Sync status: ${status.chain.syncStatus}`);
```

## Internal Implementation

The `getStatus` endpoint is implemented in the `getStatusHandlerFactory.js` file.

### Implementation Details

1. **Caching Mechanism**
   - Implements a caching strategy to reduce load on Platform subsystems
   - Cache is valid for 3 minutes by default
   - Cache is invalidated when new platform blocks are detected

2. **Data Collection**
   - Fetches status information from multiple sources in parallel:
     - Drive client status
     - Tenderdash status through RPC
     - Tenderdash network information
   - Uses `Promise.allSettled` to handle potential failures in any subsystem
   - Constructs a comprehensive response from all available data

3. **Status Determination**
   - Calculates sync status based on current height vs. latest known height
   - Provides sync progress information when syncing
   - Includes chain, network, and time information

4. **Partial Results**
   - Handles scenarios where some components are unavailable
   - Returns partial status information when possible
   - Provides fallback values for non-critical fields

5. **Dependencies**
   - Drive client for Drive status
   - Tenderdash RPC for blockchain status
   - Tenderdash networking for peer information

### Code Flow

```
Client Request 
  → gRPC Server 
    → getStatusHandler 
      → Check Cache
        → If Valid: Return Cached Status
        → If Invalid: 
          → Parallel Requests (Promise.allSettled):
            → Drive Status
            → Tenderdash Status
            → Tenderdash Network Info
          → Process All Results
          → Handle Any Failed Requests
          → Calculate Derived Fields
          → Cache Response
          → Return Complete Status
```

### Response Construction

The handler carefully builds a comprehensive status response:

1. **Version Information**
   - Protocol version: Consensus protocol version number
   - Software version: Implementation version string

2. **Time Information**
   - Block time: Timestamp of the most recent block
   - Genesis time: When the blockchain started
   - Local time: Current node's time (for comparison)

3. **Chain Information**
   - Current height and hash information
   - Sync status determination (SYNCING or READY)
   - Estimated blocks per day calculation

4. **Network Status**
   - Peer count and listening status
   - Chain ID for network identification
   - Node identifiers for the current node

This comprehensive status provides clients with a complete picture of the platform's current state and health.