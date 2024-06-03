[Back to the main page](/README.md)

## API Reference

### Table of Contents

- [Layer 1 endpoints](#layer-1-endpoints)

    - [generate](#generate)
    - [getBestBlockHash](#getbestblockhash)
    - [getBlockHash](#getblockhash)
    - [getMnListDiff](#getmnlistdiff)

## Layer 1 endpoints

### getBestBlockHash

Returns best block hash (hash of the chaintip)

*takes no arguments*

##### Response

| name         | type             | description                            |
|--------------|------------------|----------------------------------------|
| blockHash  | promise (string) | hash of chaintip                 |

---

### getBestBlockHeight

Returns best block height (height of the chaintip)

*takes no arguments*

##### Response

| name   | type             | description        |
|--------|------------------|--------------------|
| height | promise (string) | height of chaintip |

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
