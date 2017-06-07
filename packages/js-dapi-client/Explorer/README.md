### Explorer : 

#### RPC :

Not fully usable right now. 

#### API (insight) :


- `SDK.Explorer.API.getStatus()` - Retrieve information `Object`. (diff, blocks...)
- `SDK.Explorer.API.getBlock(hash|height)` - Retrieve block information `Object` from either an hash `String` or an height `Number`
   It worth mentioning that retrieving from height is slower (2 call) than from an hash you might want to use Blockchain method instead.
- `SDK.Explorer.API.getLastBlockHash(hash)` - Retrieve last block hash `String`.
- `SDK.Explorer.API.getHashFromHeight(height)` - Retrieve hash value `String` from an height `Number|String`.
- `SDK.Explorer.API.getBlockHeaders(hash|height, [nbBlocks,[direction]])` - Retrieve 25 or `Number` of block headers `Array[Object]` from an height `Number` or a Hash `String` in a `Number` direction (see exemple below).
    This feature is not propagated everywhere yet. It has been pushed some weeks ago but even our official insight api do not reflect it - yet.

###### Aliases (Will return a value using caching of fetching). 

- `SDK.Explorer.API.getLastBlockHeight()` - Retrieve the last height `Number`.
- `SDK.Explorer.API.getLastDifficulty()` - Retrieve the last diff `Number`.(float)
- `SDK.Explorer.API.getHeightFromHash(hash)` - Retrieve hash value `Number` from an hash `String`.
- `SDK.Explorer.API.getBlockConfirmations(hash|height)` - Retrieve the `Number` of confirmations of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockSize(hash|height)` - Retrieve the size `Number` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockBits(hash|height)` - Retrieve the bits `String` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockChainwork(hash|height)` - Retrieve the chainwork `String` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockMerkleRoot(hash|height)` - Retrieve the merkle root `String` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockTransactions(hash|height)` - Retrieve the transactions `Array[String]` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockTime(hash|height)` - Retrieve the timestamp (epoch in sec) `Number` of any block height `Number` or block hash `String`.
- `SDK.Explorer.API.getBlockVersion(hash|height)` - Retrieve the version `Number` of any block height `Number` or block hash `String`.