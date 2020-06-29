# Dash SDK

[![NPM Version](https://img.shields.io/npm/v/dash)](https://www.npmjs.org/package/dash)
[![Build Status](https://img.shields.io/travis/com/dashevo/dashjs)](https://travis-ci.com/dashevo/dashjs)
[![Release Date](https://img.shields.io/github/release-date/dashevo/dashjs)](https://img.shields.io/github/release-date/dashevo/dashjs)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)

Dash library allows you to transact on L1 or fetch/register documents on L2 within a single library, including management and signing of your documents.

## Table of Contents
- [Install](#install)
- [Usage](#usage)
- [Dependencies](#dependencies)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Install

### ES5/ES6 via NPM

In order to use this library, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type : `npm install dash` in your terminal.

```sh
npm install dash
```


### CDN Standalone

For browser usage, you can also directly rely on unpkg : 

```
<script src="https://unpkg.com/dash"></script>
```

## Usage

```js
const Dash = require("dash");

const client = new Dash.Client({
  network: "testnet",
  wallet: {
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
  },
});

client.wallet.getAccount().then((account) => {
  console.log("Funding address", account.getUnusedAddress().address);
  console.log("Confirmed Balance", account.getConfirmedBalance());
  client.disconnect();
});
```

## Dependencies 

The Dash SDK works using multiple dependencies that might interest you:
- [Wallet-Lib](https://github.com/dashevo/wallet-lib) - Wallet management for handling, signing and broadcasting transactions (BIP-44 HD).
- [Dashcore-Lib](https://github.com/dashevo/dashcore-lib) - Provides the main L1 blockchain primitives (Block, Transaction,...).
- [DAPI-Client](https://github.com/dashevo/dapi-client) - Client library for accessing DAPI endpoints.
- [DPP](https://github.com/dashevo/js-dpp) - Implementation (JS) of Dash Platform Protocol.

Some features might be more extensive in those libs, as Dash SDK only wraps around them to provide a single interface that is easy to use (and thus has less features).

## Documentation

More extensive documentation available at https://dashevo.github.io/DashJS/.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/DashJS/issues/new) or submit PRs.

## License

[MIT](/LICENSE) © Dash Core Group, Inc.
