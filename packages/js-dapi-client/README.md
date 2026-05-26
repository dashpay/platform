# DAPI Client

[![NPM Version](https://img.shields.io/npm/v/@dashevo/dapi-client)](https://www.npmjs.com/package/@dashevo/dapi-client)
[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashpay/platform)](https://github.com/dashpay/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

Client library used to access Dash DAPI endpoints

This library enables HTTP-based interaction with the Dash blockchain and Dash
Platform via the decentralized API ([DAPI](https://github.com/dashevo/dapi))
hosted on Dash masternodes.

 - `DAPI-Client` provides automatic server (masternode) discovery using either a default seed node or a user-supplied one
 - `DAPI-Client` maps to DAPI's [RPC](https://github.com/dashpay/platform/tree/master/packages/dapi/lib/rpcServer/commands) and [gRPC](https://github.com/dashpay/platform/tree/master/packages/dapi/lib/grpcServer/handlers) endpoints

## Table of Contents
- [Install](#install)
- [Usage](#usage)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
npm install @dashevo/dapi-client
```

### Browser usage

Response objects expose byte-valued fields as `Uint8Array` (not Node `Buffer`).
You can convert via `DAPIClient.bytes`:

```javascript
const DAPIClient = require('@dashevo/dapi-client');
const { bytesToHex, hexToBytes } = DAPIClient.bytes;

const hex = bytesToHex(proof.getQuorumHash());
```

A small number of internal code paths still construct `Buffer` instances —
notably `BlockHeadersProvider` (uses `dashcore-lib.BlockHeader` for SPV
parsing) and the `Identifier` constructor inside wasm-dpp. Node has `Buffer` built in;
browser bundlers (Vite, esbuild, webpack 5) typically auto-shim it when the
`buffer` package is installed, or you can polyfill explicitly:

```javascript
import { Buffer } from 'buffer';
globalThis.Buffer = Buffer;
```

This requirement will go away once
[dashpay/dashcore-lib#315](https://github.com/dashpay/dashcore-lib/pull/315)
(widening `BufferReader` to accept `Uint8Array`) lands and is picked up here.
Until then, browser consumers must ensure a `Buffer` global is reachable at
runtime.

## Usage

### Basic

```javascript
const DAPIClient = require('@dashevo/dapi-client');
const client = new DAPIClient();

client.core.getStatus().then((coreStatus) => {
  console.dir(coreStatus);
});
```

### Custom seed node

Custom seed nodes are necessary for connecting the client to devnets since the client library is unaware of them otherwise.

```javascript
const DAPIClient = require('@dashevo/dapi-client');

var client = new DAPIClient({
  seeds: [{
     host: 'seed-1.evonet.networks.dash.org',
     port: 443,
  }],
});

client.core.getBestBlockHash().then((r) => {
  console.log(r);
});
```

**Note**: The seed node shown above (`seed-1.evonet.networks.dash.org`) is for the Dash Evonet testing network.

### Custom addresses

Custom addresses may be directly specified in cases where it is beneficial to know exactly what node(s) are being accessed (e.g. debugging, local development, etc.).

```javascript
const DAPIClient = require('@dashevo/dapi-client');

var client = new DAPIClient({
  dapiAddresses: [
    '127.0.0.1:443',
    '127.0.0.2:443',
  ],
});

client.core.getBestBlockHash().then((r) => {
  console.log(r);
});
```

### Command specific options

DAPI Client options can be passed directly to any command to override any predefined client options and modify the client's behavior for that specific call.

```javascript
const DAPIClient = require('@dashevo/dapi-client');

// Set options to direct the request to a specific address and disable retries
const options = {
  dapiAddresses: ['127.0.0.1'],
  retries: 0,
};

client.core.getBestBlockHash(options).then((r) => {
  console.log(r);
});
```

## Documentation

More extensive documentation available at https://dashpay.github.io/platform/DAPI-Client/.


## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
