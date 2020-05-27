## Dash SDK

[![Package Version](https://img.shields.io/github/package-json/v/dashevo/dashjs.svg?&style=flat-square)](https://www.npmjs.org/package/dash)
[![Build Status](https://img.shields.io/travis/com/dashevo/dashjs.svg?branch=master&style=flat-square)](https://travis-ci.com/dashevo/dashjs)

> Client-side library for wallet payment/signing and application development with Dash. (Wallet, DAPI, Primitives, BLS, ...)

---

Dash SDK is intended to provide, in a single entry-point all the different features, classes & utils you might need to interface with the Dash network.

## Install

## Browser 

```html
<script src="https://unpkg.com/dash"></script>
```

## Node

In order to use this library, you will need to add our [NPM package](https://www.npmjs.com/dash) to your project.

Having [NodeJS](https://nodejs.org/) installed, just type :

```bash
npm install dash
```

### Usage 

```
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


### Use-cases examples
- [Generate a mnemonic](/examples/generate-a-new-mnemonic.md) 
- [Receive money and display balance](/examples/receive-money-and-check-balance.md) 
- [Pay to another address](/examples/pay-to-another-address.md) 
- [Use a local evonet](/examples/use-local-evonet.md) 
- [Publishing a new contract](/examples/publishing-a-new-contract.md) 
- [Use another BIP44 account](/examples/use-different-account.md) 
    
### Tutorial
- [Register an identity](https://dashplatform.readme.io/docs/tutorial-register-an-identity)
- [Register a Name for an Identity](https://dashplatform.readme.io/docs/tutorial-register-a-name-for-an-identity)
    

## Licence

[MIT](https://github.com/dashevo/dashjs/blob/master/LICENCE.md) © Dash Core Group, Inc.

