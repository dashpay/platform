<h1 align="center">DAPI</h1>

<div align="center">
  <strong>A Dash decentralized API</strong>
</div>
<br />
<div align="center">
  <!-- Stability -->
  <a href="https://nodejs.org/api/documentation.html#documentation_stability_index">
    <img src="https://img.shields.io/badge/stability-stable-green.svg?style=flat-square"
      alt="API stability" />
  </a>
  <!-- Build Status -->
  <a href="https://travis-ci.com/dashevo/dapi">
    <img src="https://img.shields.io/travis/dashevo/dapi/master.svg?style=flat-square" alt="Build Status" />
  </a>
  <!-- NPM version -->
  <a href="https://npmjs.org/package/dapi">
    <img src="https://img.shields.io/npm/v/dapi.svg?style=flat-square" alt="NPM version" />
  </a>
</div>

## Developer Notes
//QDEVTEMP comments refers to temporary code in order to simulate a _testnet_ of dapi nodes to makes testing easier.

## Table of Content
- [Getting Started](#getting-started)
    - [Prerequisites](#prerequisites)
    - [Install DAPI](#install-dapi)
- [Quorum API](#quorum-api)
- [Payment API](#payment-api)
    - [Blocks](#blocks)
    - [Transactions](#transactions)
    - [Addresses](#addresses)
    - [Auth](#auth)
    - [Utils](#utils)
- [License](https://github.com/dashevo/dapi/blob/master/LICENSE)

## Getting Started

###  Prerequisites
##### IPFS Deamon

Install IPFS binaries from https://ipfs.io/docs/install/  
In a terminal window execute $ _ipfs daemon --enable-pubsub-experiment_  

**alternatively follow docker installation at https://github.com/dashevo/dashdrive/blob/master/docs/dev-setup.md**  

### Install DAPI

```bashl
npm install dapi@latest
bitcore-node-dash start
```

## Quorum API
```
  /quorum
```

## Payment API

When started, DAPI will listen on the port 3000.  
Many routes mimic Insight-API, therefore you might want to check the [Insight-API Documentation](https://github.com/dashevo/insight-api-dash)

### Blocks
##### Blocks list
```
  /blocks
```
##### Block
```
  /block/:hash
```
##### Block Index
```
  /block-index/:height
```
##### Raw Block
```
  /rawblock/:blockHash
```

### Transactions
##### Transaction
```
  /tx/:txid
```
##### Send transaction
```
  /tx/send
```
##### Send Instant Transaction
```
  /tx/sendix
```

### Addresses
##### Get Address
```
  /addr/:addr
```
##### Get Address property 
```
  /addr/:addr/balance
  /addr/:addr/totalReceived
  /addr/:addr/totalSent
  /addr/:addr/unconfirmedBalance
  /addr/:addr/utxo
```
##### Get Addresses property 
:addrs are comma separated
```
  /addrs/:addrs/utxo
  /addrs/:addrs/tx
```

### Auth
##### Challenge
```
  /auth/challenge/:identifier
```
### Utils
##### Estimate fee
```
  /utils/estimatefee
```
##### Currency
```
  /currency
```
##### Status
```
  /status
```
##### Sync
```
  /sync
```
##### Peer
```
  /peer
```
##### Version
```
  /version
```
##### Masternodes List
```
  /masternodes/list
```




