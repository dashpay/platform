# Wallet Library

[![Package Version](https://img.shields.io/github/package-json/v/dashevo/wallet-lib.svg?&style=flat-square)](https://www.npmjs.org/package/@dashevo/wallet-lib)
[![Build Status](https://img.shields.io/travis/com/dashevo/wallet-lib.svg?branch=master&style=flat-square)](https://travis-ci.com/dashevo/wallet-lib)

> A pure and extensible JavaScript Wallet Library for Dash

Find more in the : 
- [Documentation](https://dashevo.github.io/wallet-lib/#/)
- [Examples & snippets](https://dashevo.github.io/wallet-lib/#/usage/examples)

## State

Under active development. 

## Principles

Dash is a powerful new peer-to-peer platform for the next generation of financial technology. The decentralized nature of the Dash network allows for highly resilient Dash infrastructure, and the developer community needs reliable, open-source tools to implement Dash apps and services.

## Install

### Node

In order to use this library, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type in your terminal : 

```sh
npm install @dashevo/wallet-lib
```

### CDN Standalone

For browser usage, you can also directly rely on unpkg. Below, we also assume you use [localForage](https://github.com/localForage/localForage) as your persistence adapter.

```
<script src="https://unpkg.com/@dashevo/wallet-lib"></script>
<script src="https://unpkg.com/localforage"></script>
const wallet = new Wallet({adapter: localforage});
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

Wallet will by default connects to DAPI and use either localforage (browser based device) or a InMem adapter.  
Account will by default be on expected BIP44 path (...0/0).

### Transports:

Insight-Client has been removed from MVP and is not working since Wallet-lib v3.0.

- [DAPI-Client](https://github.com/dashevo/dapi-client)

### Adapters :

- [LocalForage](https://github.com/localForage/localForage)
- [ReactNative AsyncStorage](https://facebook.github.io/react-native/docs/asyncstorage)

## Examples

You can see here, some [Examples](/docs/usage/examples.md).

## Credits

Wallet-Lib is maintained by the Dash Core Developers.
We want to thank all member of the community that have submitted suggestions, issues and pull requests.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
