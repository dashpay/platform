# subscribeToMasternodeList

## Client API

Subscribes to masternode list updates.

**Request Parameters**: None

**Response Parameters**:
- Stream of `masternodeListDiff` messages with masternode list differences

**Example Usage**:
```javascript
const stream = dapiClient.core.subscribeToMasternodeList();

stream.on('data', (masternodeListDiff) => {
  console.log('Masternode list updated:', masternodeListDiff);
});
```

## Internal Implementation

The `subscribeToMasternodeList` endpoint is implemented in the `subscribeToMasternodeListHandlerFactory.js` file.

### Implementation Details

1. **Masternode List Sync**
   - Uses the `MasternodeListSync` class to maintain a synchronized view of the masternode list
   - Listens for updates from Core node via RPC and ZMQ

2. **Streaming Mechanism**
   - Establishes a gRPC server-side streaming connection
   - Emits masternode list differences when changes are detected
   - Client receives a stream of diff updates rather than full list snapshots

3. **Diff Calculation**
   - Calculates only the changes between previous and current masternode lists
   - Includes added, removed, and modified masternodes
   - Optimizes bandwidth by sending only what has changed

4. **Dependencies**
   - Dash Core RPC interface for initial list retrieval
   - ZMQ for change notifications
   - MasternodeListSync for maintaining state

5. **Error Handling**
   - Handles disconnections from Core
   - Provides reconnection logic
   - Propagates errors to the client through the stream

### Code Flow

```
Client Request 
  → gRPC Server 
    → subscribeToMasternodeListHandler 
      → Initialize Stream Response
      → Get Initial Masternode List
      → Send Initial List to Client
      → Subscribe to Masternode List Updates
        → On Update:
          → Calculate Diff from Previous List
          → Send Diff to Client
      → On Client Disconnect:
        → Clean up Subscriptions
```

### Masternode List Diffs Structure

The diff updates contain several key components:

1. **Base Block Hash**
   - The block hash where this diff starts

2. **Block Hash**
   - The block hash where this diff ends

3. **Added Masternodes**
   - List of newly activated masternodes

4. **Modified Masternodes**
   - List of existing masternodes with changed properties

5. **Removed Masternodes**
   - List of masternodes that are no longer active

This structure allows clients to efficiently maintain their own copy of the masternode list by applying these diffs to their local state.