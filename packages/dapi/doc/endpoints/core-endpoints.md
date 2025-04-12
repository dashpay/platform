# DAPI Core Endpoints

These endpoints provide access to the Dash Core blockchain functionality through the gRPC interface.

## Available Endpoints

### `getBestBlockHeight`

Returns the height of the latest block in the chain.

**Request Parameters**: None

**Response Parameters**:
- `height`: Height of the best block (number)

**Example Usage**:
```javascript
const { getBestBlockHeight } = dapiClient.core;
const height = await getBestBlockHeight();
console.log(`Current block height: ${height}`);
```

### `getBlockchainStatus`

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
const { getBlockchainStatus } = dapiClient.core;
const status = await getBlockchainStatus();
console.log(`Network: ${status.chain}, Blocks: ${status.blocks}`);
```

### `getTransaction`

Retrieves a transaction by ID.

**Request Parameters**:
- `id`: Transaction ID (string, required)

**Response Parameters**:
- `transaction`: Binary buffer containing serialized transaction
- `blockHash`: Hash of the block containing the transaction
- `height`: Height of the block containing the transaction
- `confirmations`: Number of confirmations
- `isInstantLocked`: Whether transaction is instant locked
- `isChainLocked`: Whether transaction is chain locked

**Example Usage**:
```javascript
const { getTransaction } = dapiClient.core;
const txId = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
const tx = await getTransaction(txId);
console.log(`Transaction is in block: ${tx.height}`);
```

### `broadcastTransaction`

Broadcasts a transaction to the Dash network.

**Request Parameters**:
- `transaction`: Binary buffer containing serialized transaction (required)

**Response Parameters**:
- `transactionId`: ID of the broadcast transaction (string)

**Example Usage**:
```javascript
const { broadcastTransaction } = dapiClient.core;
const txHex = '0200000001...'; // Raw transaction hex
const result = await broadcastTransaction(txHex);
console.log(`Transaction broadcast with ID: ${result.transactionId}`);
```

## Stream Endpoints

These endpoints are provided by the Core Streams Process.

### `subscribeToMasternodeList`

Subscribes to masternode list updates.

**Request Parameters**: None

**Response Parameters**:
- Stream of `masternodeListDiff` messages with masternode list differences

**Example Usage**:
```javascript
const { subscribeToMasternodeList } = dapiClient.core;
const stream = subscribeToMasternodeList();

stream.on('data', (masternodeListDiff) => {
  console.log('Masternode list updated:', masternodeListDiff);
});
```

### `subscribeToTransactionsWithProofs`

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
- Stream of transactions, merkle blocks, and instant lock messages

**Example Usage**:
```javascript
const { subscribeToTransactionsWithProofs } = dapiClient.core;

const bloomFilter = {
  vData: '...',
  nHashFuncs: 11,
  nTweak: 0,
  nFlags: 0
};

const stream = subscribeToTransactionsWithProofs(bloomFilter);

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

### `subscribeToBlockHeadersWithChainLocks`

Streams block headers and chain locks.

**Request Parameters**:
- `fromBlockHash` or `fromBlockHeight`: Starting point (optional)
- `count`: Number of blocks to fetch (0 means subscribe to new blocks)

**Response Parameters**:
- Stream of block headers and chain locks

**Example Usage**:
```javascript
const { subscribeToBlockHeadersWithChainLocks } = dapiClient.core;

const stream = subscribeToBlockHeadersWithChainLocks({ fromBlockHeight: 1000, count: 10 });

stream.on('data', (response) => {
  if (response.rawBlockHeader) {
    console.log('Received block header');
  } else if (response.rawChainLock) {
    console.log('Received chain lock');
  }
});
```