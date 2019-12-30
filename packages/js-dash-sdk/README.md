# DashJS

[![Package Version](https://img.shields.io/github/package-json/v/dashevo/dashjs.svg?&style=flat-square)](https://www.npmjs.org/package/dash)
[![Build Status](https://img.shields.io/travis/com/dashevo/dashjs.svg?branch=master&style=flat-square)](https://travis-ci.com/dashevo/dashjs)

> Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)


DashJS allows you to transact on L1 or fetch/register documents on L2 within a single library, including management and signing of your documents.

Find more in the : 
- [Documentation](https://dashevo.github.io/DashJS/#/)
- [Examples & snippets](https://dashevo.github.io/DashJS/#/)

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
import DashJS from "dash"; 
import schema from "./schema.json";

const network = "testnet";
const opts = {
    network,
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
    schema
};
const sdk = new DashJS.SDK(opts);
const account = sdk.wallet.getAccount();
async function sendPayment(){
    const txOpts = {recipient:{address:'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h', amount:0.12}};
    const tx = await account.createTransaction(txOpts)
    console.log(tx)
}

async function readDocument() {
    const profile = await sdk.platform.fetchDocuments('profile',{name:'Bob'})
    console.log(profile);
}
```

## In a nutshell 

- If you use multiple contracts, fetchDocuments is done using dot-locator `dashpay.profile` and passing a named-schemas object.
   See more on how to [work with multiple contracts in detail](https://dashevo.github.io/DashJS/#/getting-started/multiples-schemas)

## Dependencies 

DashJS works using multiple dependencies that might interest you :
- [Wallet-Lib](https://github.com/dashevo/wallet-lib) - Wallet management for handling, signing and broadcasting transactions (HD44).
- [Dashcore-Lib](https://github.com/dashevo/dashcore-lib) - Providing the main primitives (Block, Transaction,...).
- [DAPI-Client](https://github.com/dashevo/dapi-client) - Client library for accessing DAPI endpoints.
- [DPP](https://github.com/dashevo/js-dpp) - Implementation (JS) of Dash Platform Protocol.

Some features might be more extensive in those libs, as DashJS only wraps around them to provide a single interface that is easy to use (and thus has less features).

## Licence

[MIT](/LICENCE.md) © Dash Core Group, Inc.
