## Wallet-lib

[![Package Version](https://img.shields.io/github/package-json/v/dashevo/wallet-lib.svg?&style=flat-square)](https://www.npmjs.org/package/@dashevo/wallet-lib)
[![Build Status](https://img.shields.io/travis/com/dashevo/wallet-lib.svg?branch=master&style=flat-square)](https://travis-ci.com/dashevo/wallet-lib)

> A pure and extensible JavaScript Wallet Library for Dash

### What it is 

From being able to display an account balance, to paying to another address, passing by the need to automate back-end task related to a cold-storage.  
The Wallet-lib allows you to easily work with Wallet/Account for HDWallet, or from just a single private key.  
Wallet-lib also allow you to watch for a public key or an HDPubKey.  
You might also wish to have your own set of plugins or your own coin selection logic.  
The wallet-lib provides all the set of feature intended for node and browser usage.  

### Install

### ES5/ES6 via NPM

In order to use this library in Node, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type in your terminal : 

```sh
npm install @dashevo/wallet-lib
```

### CDN Standalone

For browser usage, you can also directly rely on unpkg :  

```
<script src="https://unpkg.com/@dashevo/wallet-lib"></script>
```


## Usage

In your file, where you want to execute it :

```js
const { Wallet, EVENTS } = require('@dashevo/wallet-lib');

const wallet = new Wallet();

// We can dump our initialization parameters
const mnemonic = wallet.exportWallet();

wallet.getAccount().then((account) => {
  // At this point, account has fetch all UTXOs if they exists
  const balance = account.getTotalBalance();
  console.log(`Balance: ${balance}`);

  // We easily can get a new address to fund
  const { address } = account.getUnusedAddress();
});
```


## Licence

[MIT](https://github.com/dashevo/wallet-lib/blob/master/LICENCE.md) © Dash Core Group, Inc.

