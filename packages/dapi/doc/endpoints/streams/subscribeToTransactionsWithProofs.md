# subscribeToTransactionsWithProofs

## Client API

Streams transactions matching a bloom filter with merkle proofs.

**Request Parameters**:
- `bloomFilter`: Parameters for bloom filter (object, required)
  - `vData`: Filter data
  - `nHashFuncs`: Number of hash functions
  - `nTweak`: Random value
  - `nFlags`: Bloom filter update flags
- `fromBlockHash` or `fromBlockHeight`: Starting point (optional)
- `count`: Number of blocks to fetch (0 means subscribe to new transactions)

**Response Parameters**:
- Stream of:
  - `rawTransaction`: Serialized transaction
  - `rawMerkleBlock`: Serialized merkle block (proof)
  - `instantSendLockMessages`: Instant send lock data

**Example Usage**:
```javascript
const bloomFilter = {
  vData: '...',
  nHashFuncs: 11,
  nTweak: 0,
  nFlags: 0
};

const stream = dapiClient.core.subscribeToTransactionsWithProofs(bloomFilter);

stream.on('data', (response) => {
  if (response.rawMerkleBlock) {
    console.log('Received merkle block');
  } else if (response.rawTransaction) {
    console.log('Received transaction');
  } else if (response.instantSendLockMessages) {
    console.log('Received instant send lock');
  }
});
```

## Internal Implementation

The `subscribeToTransactionsWithProofs` endpoint is implemented in the `subscribeToTransactionsWithProofsHandlerFactory.js` file.

### Implementation Details

1. **Bloom Filtering**
   - Uses Bloom filters to efficiently match transactions of interest
   - Applies filter to both historical and new transactions
   - Supports standard Bloom filter flags for filter updates

2. **Operation Modes**
   - **Historical Mode**: Retrieves transactions from past blocks
   - **Streaming Mode**: Provides real-time transaction updates
   - **Combined Mode**: Historical data followed by real-time streaming

3. **Historical Data Processing**
   - Creates an iterator to efficiently process historical blocks
   - For each block, filters transactions using the provided Bloom filter
   - Sends matching transactions along with Merkle proofs
   - Sends associated InstantLock messages when available

4. **Real-time Streaming**
   - Uses a `ProcessMediator` for event-driven architecture
   - Subscribes to Core ZMQ events for new transactions and blocks
   - Tests each transaction against the filter before sending
   - Includes Merkle proofs for transaction inclusion verification

5. **Mempool Handling**
   - Processes mempool transactions to catch recent unconfirmed transactions
   - Applies rate limiting to reduce load on Core
   - Sends mempool transactions that match the filter

6. **Dependencies**
   - ZMQ client for real-time notifications
   - Bloom filter implementation for transaction filtering
   - ProcessMediator for event management
   - Block cache for efficient block retrieval

### Code Flow

```
Client Request 
  → gRPC Server 
    → subscribeToTransactionsWithProofsHandler 
      → Validate Bloom Filter Parameters
      → Create BloomFilter Instance
      → If Historical Data Requested (count > 0):
        → Process Mempool Transactions
        → Create Historical Transactions Iterator
        → For Each Block:
          → Filter Transactions Using Bloom Filter
          → Send Matching Transactions
          → Send Merkle Block (Proof)
          → Send Associated InstantLocks
      → If Streaming Requested (count = 0 or all historical sent):
        → Subscribe to ZMQ Events
        → On New Block:
          → Filter Transactions
          → Send Matching Transactions with Proofs
        → On New Transaction:
          → Test Against Filter
          → If Match: Send Transaction
      → On Client Disconnect:
        → Clean Up Resources
```

### Response Types

The endpoint emits three types of responses:

1. **Raw Transactions**
   - Serialized transaction data for transactions matching the filter
   - Includes transaction metadata

2. **Merkle Blocks**
   - Partial block data that proves transaction inclusion
   - Contains Merkle path to verify transactions without downloading the entire block

3. **InstantSend Locks**
   - Provides instant transaction finality information
   - Allows clients to know when a transaction is secured by InstantSend

These responses collectively enable lightweight clients to validate transactions without downloading the full blockchain.