[Back to the main page](/README.md)

## API Reference

### Table of Contents

- [Layer 1 endpoints](#layer-1-endpoints)

    - [addToBloomFilter](#addtobloomfilter)
    - [clearBloomFilter](#clearbloomfilter)
    - [estimateFee](#estimatefee)
    - [findDataForBlock](#finddataforblock)
    - [generate](#generate)
    - [getAddressSummary](#getaddresssummary)
    - [getAddressTotalReceived](#getaddresstotalreceived)
    - [getAddressTotalSent](#getaddresstotalsent)
    - [getAddressUnconfirmedBalance](#getaddressunconfirmedbalance)
    - [getAuthChallenge](#getauthchallenge)
    - [getBalance](#getbalance)
    - [getBestBlockHash](#getbestblockhash)
    - [getBestBlockHeight](#getbestblockheight)
    - [getBlockHash](#getblockhash)
    - [getBlockHeaders](#getblockheaders)
    - [getBlocks](#getblocks)
    - [getCurrency](#getcurrency)
    - [getHistoricBlockchainDataSyncStatus](#gethistoricblockchaindatasyncstatus)
    - [getMempoolInfo](#getmempoolinfo)
    - [getMNList](#getmnlist)
    - [getMnListDiff](#getmnlistdiff)
    - [getMNUpdateList](#getmnupdatelist)
    - [getPeerDataSyncStatus](#getpeerdatasyncstatus)
    - [getRawBlock](#getrawblock)
    - [getSpvData](#getspvdata)
    - [getStatus](#getstatus)
    - [getTransactionById](#gettransactionbyid)
    - [getTransactionsByAddress](#gettransactionsbyaddress)
    - [getUTXO](#getutxo)
    - [getVersion](#getversion)
    - [loadBloomFilter](#loadbloomfilter)
    - [sendRawTransaction](#sendrawtransaction)

 - [Layer 2 endpoints](#Layer-2-endpoints)

    - [fetchDapContract](#fetchdapcontract)
    - [getUser](#getuser)
    - [searchUsers](#searchusers)
    - [sendRawTransition](#sendrawtransition)


## Layer 1 endpoints
### addToBloomFilter

Adds something to bloom filter

##### Params

| name                 | type   | description                            |
|----------------------|--------|----------------------------------------|
| args.originalFilter  | string | Original bloom filter                  |
| args.element         | string | Element to add to bloom filter         |

##### Response

| name     | type              | description                                                      |
|----------|-------------------|------------------------------------------------------------------|
| headers  | promise (boolean) | Returns a boolean promise indicating whether it was added or not |

---

### clearBloomFilter

Clears bloom filter

##### Params

| name         | type   | description                                 |
|--------------|--------|---------------------------------------------|
| args.filter  | string | Bloom filter that you want to clear         |

##### Response

| name    | type              | description                                                                  |
|---------|-------------------|------------------------------------------------------------------------------|
| headers | promise (boolean) | Returns a boolean promise indicating whether it was cleared or not           |

---

### estimateFee

Estimates transaction fee based on the size of number of blocks.

##### Params

| name          | type   | description                            |
|---------------|--------|----------------------------------------|
| args.nBlocks  | number | Number of blocks for fee estimate      |

##### Response

| name         | type             | description                                  |
|--------------|------------------|----------------------------------------------|
| estimatedFee | promise (number) | Promise containing fee in duffs per kilobyte |

---

### findDataForBlock

Finds data for blocks given a bloom filter.

##### Params

| name         | type   | description                            |
|--------------|--------|----------------------------------------|
| args.filter  | string | A bloom filter                         |

##### Response

| name    | type   | description                             |
|---------|--------|-----------------------------------------|
| headers | object | Returns an obj containing block headers |

---

### generate *(regest only)*

Generates blocks on demand for regression tests.

##### Params

| name    | type   | description                            |
|---------|--------|----------------------------------------|
| args.amount  | number | Amount of blocks to generate |

##### Response

| name         | type                   | description                                                |
|--------------|------------------------|------------------------------------------------------------|
| blockHashes  | promise (string array) | Returns a promise containing strings of block hashes       |

---

### getAddressSummary

Returns an address summary given an address.

##### Params

| name            | type                   | description                            |
|-----------------|------------------------|----------------------------------------|
| args.address    | string or string array | address or multiple addresses          |
| args.noTxList   | boolean                | set true if no list txs needed (opt.)  |
| args.from       | number                 | start of range (opt.)                  |
| args.to         | number                 | end of range (opt.)                    |
| args.fromHeight | number                 | which height to start from (opt.)      |
| args.toHeight   | number                 | which height to end (opt.)             |

##### Response

| name           | type             | description                                              |
|----------------|------------------|----------------------------------------------------------|
| addressSummary | promise (object) | Returns a promise containing an obj with summary details |

---

### getAddressTotalReceived

Returns total amount of duffs received by an address.

##### Params

| name            | type                   | description                     |
|-----------------|------------------------|---------------------------------|
| args.address    | string or string array | address or multiple addresses   |

##### Response

| name           | type             | description                            |
|----------------|------------------|----------------------------------------|
| totalReceived  | promise (number) | Number of duffs received by address    |

------

### getAddressTotalSent

Returns total amount of duffs sent by an address.

##### Params

| name            | type                   | description                     |
|-----------------|------------------------|---------------------------------|
| args.address    | string or string array | address or multiple addresses   |

##### Response

| name       | type             | description                            |
|------------|------------------|----------------------------------------|
| totalSent  | promise (number) | number of duffs sent by address        |

---

### getAddressUnconfirmedBalance

Returns total unconfirmed balance for a given address.

##### Params

| name            | type                   | description                     |
|-----------------|------------------------|---------------------------------|
| args.address    | string or string array | address or multiple addresses   |

##### Response

| name                | type             | description                            |
|---------------------|------------------|----------------------------------------|
| unconfirmedBalance  | promise (number) | an address's unconfirmed balance       |

---

### getBalance

Returns calculated balance for an address.

##### Params

| name            | type                   | description                     |
|-----------------|------------------------|---------------------------------|
| args.address    | string or string array | address or multiple addresses   |

##### Response

| name          | type             | description                            |
|---------------|------------------|----------------------------------------|
| calcBalance   | promise (number) | promise containing calculated balance  |

---

### getBestBlockHash

Returns best block hash (hash of the chaintip)

*takes no arguments*

##### Response

| name         | type             | description                            |
|--------------|------------------|----------------------------------------|
| blockHash  | promise (string) | hash of chaintip                 |

---

### getBestBlockHeight

Returns best block height

*takes no arguments*

##### Response

| name         | type             | description                            |
|--------------|------------------|----------------------------------------|
| blockHeight  | promise (number) | best seen block height                 |

---

### getBlockHash

Returns block hash for a given height.

##### Params

| name         | type   | description                            |
|--------------|--------|----------------------------------------|
| args.height  | number | block height                           |

##### Response

| name       | type             | description                                 |
|------------|------------------|---------------------------------------------|
| blockHash  | promise (string) | promise containing a string of a block hash |

---

### getBlockHeaders

Returns block headers

##### Params

| name        | type   | description                            |
|-------------|--------|----------------------------------------|
| args.offset | number | block height starting point            |
| args.limit  | number | number of block headers to return      |

##### Response

| name         | type                | description                                  |
|--------------|---------------------|----------------------------------------------|
| blockHeaders | promise (obj array) | promise containing an array of block headers |

---

### getBlocks

Returns info for blocks.

##### Params

| name           | type   | description                            |
|----------------|--------|----------------------------------------|
| args.blockDate | string | Starting date for blocks to get        |
| args.limit     | number | Number of blocks to return             |

##### Response

| name    | type                | description                            |
|---------|---------------------|----------------------------------------|
| blocks  | promise (obj array) | array of block objs                    |

---

### getHistoricBlockchainDataSyncStatus

Returns historic blockchain data sync status.

*Takes no arguments*

##### Response

| name            | type          | description                              |
|-----------------|---------------|------------------------------------------|
| historicStatus  | promise (obj) | object containing historical sync status |

---

### getMempoolInfo

Returns historic blockchain data sync status.

*Takes no arguments*

##### Response

| name            | type          | description                              |
|-----------------|---------------|------------------------------------------|
| mempoolInfo  | promise (obj) | object containing mempool info |

---

### getMNList

Returns masternode list.

*Takes no arguments*

##### Response

| name    | type                | description                            |
|---------|---------------------|----------------------------------------|
| mnList  | promise (obj array) | an array of masternodes                |

---

### getMnListDiff

*needs definition*

##### Params

| name    | type   | description                            |
|---------|--------|----------------------------------------|
| packet  | string | ST Packet object serialized using CBOR |

##### Response

| name    | type   | description                            |
|---------|--------|----------------------------------------|
| packet  | string | ST Packet object serialized using CBOR |

---

### getPeerDataSyncStatus

Returns peer data sync status.

*Takes no arguments*

##### Response

| name                | type   | description                                    |
|---------------------|---------------|-----------------------------------------|
| peerDataSyncStatus  | promise (obj) | object containing peer data sync status |

---

### getRawBlock

Returns raw block given block hash.

##### Params

| name            | type   | description                            |
|-----------------|--------|----------------------------------------|
| args.blockHash  | string | block hash                             |

##### Response

| name     | type          | description                             |
|----------|---------------|-----------------------------------------|
| rawBlock | promise (obj) | object containing the raw block details |

---

### getSpvData

Returns block headers.

##### Params

| name         | type   | description                            |
|--------------|--------|----------------------------------------|
| args.filter  | string | bloom filter                           |

##### Response

| name         | type   | description                            |
|--------------|--------|----------------------------------------|
| blockHeaders | object | object containing block headers        |

---

### getTransactionById

Returns tranasction for the given hash.

##### Params

| name      | type   | description                            |
|-----------|--------|----------------------------------------|
| args.txid | string | transaction id                         |

##### Response

| name    | type          | description                            |
|---------|---------------|----------------------------------------|
| tx      | promise (obj) | object containing transaction info     |

---

### getTransactionsByAddress

Returns all transactions for a given address.

##### Params

| name            | type                   | description                        |
|-----------------|------------------------|------------------------------------|
| args.address    | string or string array | address or multiple addresses      |
| args.from       | number                 | start of range (opt.)              |
| args.to         | number                 | end of range (opt.)                |
| args.fromHeight | number                 | which height to start from (opt.)  |
| args.toHeight   | number                 | which height to end (opt.)         |

##### Response

| name       | type                | description                                |
|------------|---------------------|--------------------------------------------|
| addressTxs | promise (obj array) | array containing tx objects for an address |

---

### getUTXO

Returns unspent transaction outputs for a given address.

##### Params

| name            | type                   | description                        |
|-----------------|------------------------|------------------------------------|
| args.address    | string or string array | address or multiple addresses      |
| args.from       | number                 | start of range (opt.)              |
| args.to         | number                 | end of range (opt.)                |
| args.fromHeight | number                 | which height to start from (opt.)  |
| args.toHeight   | number                 | which height to end (opt.)         |

##### Response

| name    | type                | description                                               |
|---------|---------------------|-----------------------------------------------|
| utxo    | promise (obj array) | an array containing unspent transaction objs  |

---

### loadBloomFilter

Loads bloom filter.

##### Params

| name        | type   | description                            |
|-------------|--------|----------------------------------------|
| args.filter | string | bloom filter                           |

##### Response

| name          | type              | description                            |
|---------------|-------------------|----------------------------------------|
| filterLoaded? | promise (boolean) | returns boolean depending on load status |

---

### sendRawIxTransaction

Sends raw instant send transaction and returns the transaction id.

##### Params

| name                  | type   | description                            |
|-----------------------|--------|----------------------------------------|
| args.rawIxTransaction | string | raw instant send transaction           |

##### Response

| name          | type             | description                                  |
|---------------|------------------|----------------------------------------------|
| transactionId | promise (string) | instant send transaction id                  |

---

### sendRawTransaction

Sends raw transaction to the network.

##### Params

| name                | type   | description                            |
|---------------------|--------|----------------------------------------|
| args.rawTransaction | string | raw transaction to be sent             |

##### Response

| name  | type             | description                            |
|-------|------------------|----------------------------------------|
| txId  | promise (string) | string of transaction id               |

## Layer 2 endpoints

### fetchDapContract

Returns user's Dap space.

##### Params

| name       | type   | description                            |
|------------|--------|----------------------------------------|
| args.contractId | string | User's contract Id                          |

##### Response

| name     | type          | description                            |
|----------|---------------|----------------------------------------|
| dapSpace | promise (obj) | User's dap space                       |

---

### getUser

Returns blockchain user

##### Params

| name                | type   | description                                          |
|---------------------|--------|------------------------------------------------------|
| username OR regTxId | string | Either provide username or user's registration tx id |

##### Response

| name    | type          | description                            |
|---------|---------------|----------------------------------------|
| user    | promise (obj) | object containing user info            |

---

### searchUsers

Returns list of users after matching search criteria.

##### Params

| name    | type   | description                               |
|---------|--------|-------------------------------------------|
| args.offset  | number | starting amount of results to return |
| args.limit   | number | limit of search results to return    |
| args.pattern | string | search pattern                       |

##### Response

| name    | type                              | description                                              |
|---------|-----------------------------------|----------------------------------------------------------|
| users   | promise (obj w/ array of strings) | obj w/ totalResults found and an array of matching users |

---

### sendRawTransition

Sends raw state transition to the network.

##### Params

| name                      | type               | description                |
|---------------------------|--------------------|----------------------------|
| args.rawStateTransition  | *Needs definition* | *needs definition*         |
| args.rawSTPacket  | *needs definition* | *needs definition*         |

##### Response

| name  | type             | description                                      |
|-------|------------------|--------------------------------------------------|
| tsId  | promise (string) | string of confirmed state transition transaction |
