# Dash SDK

[![Package Version](https://img.shields.io/github/package-json/v/dashevo/dashjs.svg?&style=flat-square)](https://www.npmjs.org/package/dash)
[![Build Status](https://img.shields.io/travis/com/dashevo/dashjs.svg?branch=master&style=flat-square)](https://travis-ci.com/dashevo/dashjs)

> Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)


Dash library allows you to transact on L1 or fetch/register documents on L2 within a single library, including management and signing of your documents.

Find more information in the [Documentation](https://dashevo.github.io/DashJS/#/).

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

client.isReady().then(async () => {
  const {account, platform} = client;
  console.log("Funding address", account.getUnusedAddress().address);
  console.log("Confirmed Balance", account.getConfirmedBalance());
  console.log(await platform.names.get('alice'));
});

```

## Dependencies 

Dash SDK works using multiple dependencies that might interest you :
- [Wallet-Lib](https://github.com/dashevo/wallet-lib) - Wallet management for handling, signing and broadcasting transactions (HD44).
- [Dashcore-Lib](https://github.com/dashevo/dashcore-lib) - Providing the main primitives (Block, Transaction,...).
- [DAPI-Client](https://github.com/dashevo/dapi-client) - Client library for accessing DAPI endpoints.
- [DPP](https://github.com/dashevo/js-dpp) - Implementation (JS) of Dash Platform Protocol.

Some features might be more extensive in those libs, as Dash SDK only wraps around them to provide a single interface that is easy to use (and thus has less features).

## Licence

[MIT](/LICENCE) © Dash Core Group, Inc.
