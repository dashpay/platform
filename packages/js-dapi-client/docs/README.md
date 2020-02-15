## DAPI-Client

[![Build Status](https://travis-ci.com/dashevo/dapi-client.svg?branch=master)](https://travis-ci.com/dashevo/dapi-client)
[![GitHub Current Tag](https://img.shields.io/github/tag-date/dashevo/dapi-client.svg)](https://github.com/dashevo/dapi-client/tags)
[![npm](https://img.shields.io/npm/v/@dashevo/dapi-client.svg)](https://www.npmjs.com/package/@dashevo/dapi-client)

> Client library used to access Dash DAPI endpoints

This library enables HTTP-based interaction with the Dash blockchain and Dash
Platform via the decentralized API ([DAPI](https://github.com/dashevo/dapi))
hosted on Dash masternodes.

 - `DAPI-Client` provides automatic server (masternode) discovery using either a default seed node or a user-supplied one
 - `DAPI-Client` maps to DAPI's [RPC](https://github.com/dashevo/dapi/tree/master/lib/rpcServer/commands) and [gRPC](https://github.com/dashevo/dapi/tree/master/lib/grpcServer/handlers) endpoints

### Install

### ES5/ES6 via NPM

In order to use this library in Node, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type in your terminal :

```sh
npm install @dashevo/dapi-client
```

### CDN Standalone

For browser usage, you can also directly rely on unpkg :

```
<script src="https://unpkg.com/@dashevo/dapi-client"></script>
```


## Licence

[MIT](https://github.com/dashevo/dapi-client/blob/master/LICENCE.md) © Dash Core Group, Inc.

