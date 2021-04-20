# Wallet Library

[![NPM Version](https://img.shields.io/npm/v/@dashevo/wallet-lib)](https://www.npmjs.com/package/@dashevo/wallet-lib)
[![Build Status](https://github.com/dashevo/wallet-lib/actions/workflows/test_and_release.yml/badge.svg)](https://github.com/dashevo/wallet-lib/actions/workflows/test_and_release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashevo/wallet-lib)](https://github.com/dashevo/wallet-lib/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

A pure and extensible JavaScript Wallet Library for Dash

## Table of Contents
- [Install](#install)
- [Usage](#usage)
- [Documentation](#documentation)
- [Maintainers](#maintainers)
- [Contributing](#contributing)
- [License](#license)


## Background

[Dash](https://www.dash.org) is a powerful new peer-to-peer platform for the next generation of financial technology. The decentralized nature of the Dash network allows for highly resilient Dash infrastructure, and the developer community needs reliable, open-source tools to implement Dash apps and services.

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

## Documentation

You can see some [examples here](/docs/usage/examples.md).

More extensive documentation is available at https://dashevo.github.io/wallet-lib along with additional [examples & snippets](https://dashevo.github.io/wallet-lib/#/usage/examples).

## Maintainers

Wallet-Lib is maintained by the [Dash Core Developers](https://www.github.com/dashevo).
We want to thank all members of the community that have submitted suggestions, issues and pull requests.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/wallet-lib/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
