# DAPI GRPC

[![Build Status](https://github.com/dashevo/dapi-grpc/actions/workflows/test_and_release.yml/badge.svg)](https://github.com/dashevo/dapi-grpc/actions/workflows/test_and_release.yml)
[![NPM version](https://img.shields.io/npm/v/@dashevo/dapi-grpc.svg)](https://npmjs.org/package/@dashevo/dapi-grpc)
[![Release Date](https://img.shields.io/github/release-date/dashevo/dapi-grpc)](https://github.com/dashevo/dapi-grpc/releases/latest)
[![license](https://img.shields.io/github/license/dashevo/dapi-grpc.svg)](LICENSE)

Decentralized API GRPC definition files and generated clients

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Install

Ensure you have the latest [NodeJS](https://nodejs.org/en/download/) installed.

#### From repository

Clone the repo:

```shell
git clone https://github.com/dashevo/dapi-grpc
```

Install npm packages:

```shell
npm install
```

#### From NPM

```sh
npm install @dashevo/dapi-grpc
```

## Usage

Node users are able to access exported elements by requiring them under v0 property.

### Core Client

Provide a client to perform core request.

```js
const {
  v0: {
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const client = new CorePromiseClient(url);
```

Provided method allow to then perform the request, by passing a specific request parameter (see below example).
All methods share the same API :
- First parameter expect a specific request instance of a Request class (such as GetBlockRequest, GetTransactionRequest).
- Second parameter is optional for metadata object.
- Third parameter is optional for options.

Here is a usage example for requesting a Block by its hash and handling its response :

```js
const {
  v0: {
    CorePromiseClient,
    GetBlockRequest,
    GetBlockResponse,
  },
} = require('@dashevo/dapi-grpc');

const client = new CorePromiseClient(url);

async function getBlockByHash(hash, options = {}) {
  const getBlockRequest = new GetBlockRequest();
  getBlockRequest.setHash(hash);

  const response = await client.getBlock(
    getBlockRequest,
    {},
    options,
  );
  const blockBinaryArray = response.getBlock();

  return Buffer.from(blockBinaryArray);
}
```

Available methods :

- getStatus
- getBlock
- broadcastTransaction
- getTransaction
- getEstimatedTransactionFee
- subscribeToBlockHeadersWithChainLocks
- subscribeToTransactionsWithProofs

For streams, such as subscribeToTransactionsWithProofs and subscribeToBlockHeadersWithChainLocks, a [grpc-web stream](https://github.com/grpc/grpc-web) will be returned.
More info on their usage can be read over their repository.

### Platform Client

Provide a client to perform platform request.
Method's API and usage is similar to CorePromiseClient.

```js
const {
  v0: {
    PlatformPromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const client = new PlatformPromiseClient(url);
```

Available methods :

- broadcastStateTransition
- getIdentity
- getDataContract
- getDocuments
- getIdentitiesByPublicKeyHashes
- getIdentityIdsByPublicKeyHashes
- waitForStateTransitionResult
- getConsensusParams
- setProtocolVersion

## Maintainer

[@shumkov](https://github.com/shumkov)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/dapi-grpc/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.

