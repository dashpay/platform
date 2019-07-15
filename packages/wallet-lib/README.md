# Wallet Library

[![NPM Package](https://img.shields.io/npm/v/@dashevo/wallet-lib.svg?style=flat-square)](https://www.npmjs.org/package/@dashevo/wallet--lib)
[![Build Status](https://img.shields.io/travis/dashevo/wallet-lib.svg?branch=master&style=flat-square)](https://travis-ci.com/dashevo/wallet-lib)
[![Coverage Status](https://img.shields.io/coveralls/dashevo/wallet-lib.svg?style=flat-square)](https://coveralls.io/github/dashevo/wallet-lib?branch=master)

> A pure and extensible JavaScript Wallet Library for Dash

## Table of Contents

- [State](#state)
- [Principles](#principles)
- [Install](#install)
- [Usage](#usage)
- [Getting Started](#getting-started)
  - [Some rules of thumb](#some-rules-of-thumb)
- [Creating a Wallet](#creating-a-wallet)
- [Creating an Account](#creating-an-account)
- [Working with an Account](#working-with-an-account)
  - [Pay to an Address](#pay-to-an-address)
- [Features](#features)
    - [BIPS Supports](#bips-supports)
    - [Miscellaneous](#miscellaneous)
    - [Transports](#transports)
    - [Adapters](#adapters)
- [API](#api)
    - [Workers](#workers)
- [Examples](#examples)
- [Road map](#road-map)
- [Credits](#credits)
- [License](#license)

## State

Under active development. Usage for production discouraged. Help from the community is also welcomed.

Things to look up for :
- More complex and edge cases for testing.
- Handling of differents transactions type + DIP002
- Storage/memory cost on a usage over multiples days.
- Optimisation allowing to work for a faucet priv Key (with a lot of inc/out tx)
- Ready event expectation in a unsuccessful initialization (timeout for instance)
- Others TODOs and FIXMEs in the code :)

## Principles

Dash is a powerful new peer-to-peer platform for the next generation of financial technology. The decentralized nature of the Dash network allows for highly resilient Dash infrastructure, and the developer community needs reliable, open-source tools to implement Dash apps and services.

## Install

```sh
npm install @dashevo/wallet-lib
```

In order to use this library, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type : `npm install @dashevo/wallet-lib` in your terminal.

## Usage

In your file, where you want to execute it :

```
const { Wallet } = require('@dashevo/wallet-lib');

const wallet = new Wallet()
const account = wallet.getAccount();

// Do something with account.
```

Wallet will by default connects to DAPI and use either localforage (browser based device) or a InMem adapter.
Account will by default be on expected BIP44 path (...0/0).

It is suggested to wait for the ready event thrown by the account to ensure every is ready to be used (in sync with plugin loaded)

```
const { Wallet, EVENTS } = require('@dashevo/wallet-lib');

const wallet = new Wallet();
const start = () => {
    const account = wallet.getAccount();
    // Do something with account.
}
account.events.on(EVENTS.READY, start);

```

## Some rules of thumb

- There is multiple event listeners(socker sync,...), running intervals (service worker,...),
therefore a good way to quit an instance would be to call `account.disconnect()` which will care to
call `clearWorker(), closeSocket()` of the differents elements. You can still decide to remove them by hand if you want.
- Some classic examples of usage can be seen here : [Examples](/docs/examples.md)

## Creating a Wallet


The goal of this library is to offer you all the helpers that you will need to handle a Wallet and it's logic.
Therefore, the only real object you want to instantiate will be a wallet object.

```
const { Wallet } = require('@dashevo/wallet-lib');
const wallet = new Wallet()
```

With no parameters passed as we did in the above sample, the Wallet Library will use it's defaults option,
in our case it mean not working with any transport layer (therefore not having the knowledge of the blockchain)
and only deal with address derivation, the cache options would allow to create transaction rawtx
(for cold-wallet/computer who does not want to connect to the network but just would love to generate some data (getUnunsedAddress),
and because of the absence of any mnemonic or seed, it will generate one for the user.

Most of the time, here is what configuration you will be using :

```
const { Wallet } = require('@dashevo/wallet-lib');
const { DAPIClient } = require('@dashevo/dapi-client');
const { Mnemonic } = require('@dashevo/dashcore-lib');
const localForage = require('localforage'); //Browser-only storage


const mnemonic = 'my mnemonic in 12 or 24 words;'; //Can also be an instance of Mnemonic
const network = 'testnet' // or 'livenet'
const transport = new DAPIClient();
const adapter = localForage.createInstance({
      name: 'persist:walletAdapter'
    });

const opts = {
    transport, mnemonic, network, adapter
};
const Wallet = new Wallet(opts);
```

For more informations about the differents methods and options of the Wallet class, look at the [Wallet documentation](/docs/wallet.md).
There is several interval running in the app (as service worker), the `disconnect()` method allow to cut them off gracefully before closing.

## Creating an Account

The Wallet object can generate multiple account for you, theses accounts follow the BIP44 account derivation.

```
const opts = {
    cacheTx: true
}
const account = wallet.createAccount(opts);
```

You also can specify a label for this account, and an accountIndex if you have the wish.
Knowing the accountIndex (by default it will be an ordering increment)

## Working with an Account

When you create an account, depending on the options passed along to Wallet and the Account,
the Wallet-library might do several things in the background (prefetch, synchronize, validate,...).

Therefore you will want to listen to the `ready` event before starting anything.

```

function doFancyStuff(){
    const balance = account.getBalance();
    const history = account.getTransactionHistory()
    const utxo = account.getUTXO();
    const unusedAddress = account.getUnusedAddress();
        // ...
}
account.events.on('ready', doFancyStuff());

```

For more informations about the differents methods and options of the Account class, look at the [Account documentation](/docs/account.md).

### Pay to an address
```
const options = {
    recipient: "yizmJb63ygipuJaRgYtpWCV2erQodmaZt8",
    satoshis:100000,
    isInstantSend:false
};
const transaction = account.createTransaction(options);
const txid = account.broadcastTransaction(transaction);
```

## Features

BIPS Supports :

- [x] BIP32 (Hierarchical Deterministic Wallets)
- [ ] BIP38 (Passphrase protected private key)
- [x] BIP39 (Mnemonic code for generating deterministic keys)
- [x] BIP43 (Purpose field for deterministic wallets)
- [x] BIP44 (Multi-Account Hierarchy for Deterministic Wallets)

Miscellaneous :

- [X] DAPI Support
- [ ] Protocol Support
- [x] Persistance interface
    - [x] In-mem support
    - [ ] DB support
- [x] HDWallet : Create from a HDPublicKey
- [x] HDWallet : Create a wallet / generate a new mnemonic
- [x] HDWallet : Validate mnemonic (BIP39)
- [x] HDWallet : Create an account
- [x] HDWallet : Address Derivation
- [x] HDWallet : Encrypt PrivKey (BIP38) - Passphrase Protection
- [x] HDWallet : Import (Mnemonic, HDPrivKey)
- [x] HDWallet : Export (Mnemonic, HDPrivKey)
- [X] PrivateKey : Single private key mngt handled
- [X] PrivateKey : WIF Supported
- [x] Discovery : Discover existing account
- [x] Discovery : Discover existing address / change
- [x] Transaction : Create transaction (rawtx)
- [x] Transaction : Broadcast transaction
- [x] Transaction : History fetching
- [x] Transaction : Balance
- [x] Transaction : InstantSend
- [x] UTXO Optimisation / CoinSelection
- [x] Fee estimation
- [ ] Bloomfilters
- [ ] Compatibility with old format (BIP32)
- [ ] Paper-sweep wallet
- [ ] Log tool to be able to help for decentralized debugging
- [X] [docs/plugins](Plugins.md) support (Worker, DPA)
- [ ] CoinSelection strategy as a plugins.
- [ ] Account network and plugins independants from each other.


## Plugins

##### DPA
- [DashPay DPA](https://github.com/dashevo/dashpay-dpa) :

##### Transports (help from community welcomed):

Insight-Client has been removed from MVP and is not working since Wallet-lib v3.0.

- [X] DAPI-Client : [https://github.com/dashevo/dapi-client]
- [ ] Dash-P2P : [src/transports/Protocol/protocolClient.js]
- [ ] DashcoreRPC : [src/transports/RPC/rpcClient.js]

##### Adapters (help from community welcomed) :

- [x] [LocalForage](https://github.com/localForage/localForage)
- [x] [ReactNative AsyncStorage](https://facebook.github.io/react-native/docs/asyncstorage)
- [ ] LocalStorage
- [ ] SecureStorage
- [ ] LevelDB
- [ ] MongoDB
- [ ] BerkeleyDB
- [ ] LowDB

## API

- [Wallet](/docs/wallet.md)
- [Account](/docs/account.md)
- [Events](/docs/events.md)
- [KeyChain](/docs/keychain.md)
- [Plugins](/docs/plugins.md)
- [Storage](/docs/storage.md)
- [CoinSelection](/docs/coinSelection.md)

### Workers

- [BIP44Worker](/docs/BIP44Worker.md)
- [SyncWorker](/docs/SyncWorker.md)

## Examples

You can see here, some [Examples](/docs/examples.md).

## Credits

Wallet-Lib is maintained by the Dash Core Developers.
We want to thanks all member of the community that have submited suggestions, issues and pull requests.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
