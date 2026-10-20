# DAPI GRPC

[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)
[![NPM version](https://img.shields.io/npm/v/@dashevo/dapi-grpc.svg)](https://npmjs.org/package/@dashevo/dapi-grpc)
[![Release Date](https://img.shields.io/github/release-date/dashpay/platform)](https://github.com/dashpay/platform/releases/latest)
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
- waitForStateTransitionResult
- getConsensusParams
- setProtocolVersion

## Maintainer

[@shumkov](https://github.com/shumkov)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.

## Building generated clients

From the Platform monorepo, run `yarn install`, then provision the native
client generators once:

```sh
python3 packages/dapi-grpc/scripts/setup-codegen.py --install
yarn workspace @dashevo/dapi-grpc build
```

Installation needs Python 3.12 or newer, CMake and a C++ compiler. It builds
checksum-verified sources into your user cache without sudo or Docker. Linux
runner images provide the same tools at `/opt/client-codegen`. Set
`DAPI_GRPC_TOOLCHAIN` to use an explicitly provisioned installation; a missing or
mismatched installation fails rather than silently selecting a different protoc.

`codegen.json` pins the recipe and native generator versions. The client compiler
is deliberately separate from the Rust build's protoc 32.0: the existing client
output uses protobuf 3.18.1, gRPC 1.46.3, gRPC Java 1.42.1 and the Yarn-locked
`ts-protoc-gen` 0.15.0. Update the recipe, Platform lock and runner requirements
together, with generated-output compatibility checks. Ordinary builds never
install system packages or start containers.

Generation stages all languages before replacing `clients/`, so a failed plugin
preserves the previous output. Java, Objective-C and Python are generated for
repository consumers; the NPM archive continues to exclude them and ships the
Node and web clients. Run the generation regressions after installing the tools:

```sh
yarn workspace @dashevo/dapi-grpc exec python3 -m unittest discover -s tests/codegen -v
```
