# Wallet Library


[![NPM Package](https://img.shields.io/npm/v/@dashevo/wallet-lib.svg?style=flat-square)](https://www.npmjs.org/package/@dashevo/wallet--lib)
[![Build Status](https://img.shields.io/travis/dashevo/wallet-lib.svg?branch=master&style=flat-square)](https://travis-ci.org/dashevo/wallet-lib)
[![Coverage Status](https://img.shields.io/coveralls/dashevo/wallet-lib.svg?style=flat-square)](https://coveralls.io/github/dashevo/wallet-lib?branch=master)

A pure JavaScript Wallet Library for Dash - BIP44 Derivation, Coin/Utxo Selection/optimisation, transactions, multi-account...

See also for specific Layer 2 needs (DashDrive/DAPs) : [DAP-SDK](https://github.com/dashevo/dap-sdk)

## State

NOT SUITABLE FOR PRODUCTION. Under active development.  
As date of writing (Sep 18), we still miss a proper testing/QA.  
Help from the community is also welcomed :)  
 
Things to look up :
- We aren't sure on what is the storage/memory cost in a usage over multiples days.
- We didn't ran it enought to be sure we cover all edges case
- Mutating objects
- Perfomance optimisation (probably on the workers)
- TODOs in the code :)


## Table of Contents

- [Principles](#principles)
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
- [Exemples](#exemples)
- [Road map](#road-map)
- [License](#license)
- [Credits](#credits)


## Principles

Dash is a powerful new peer-to-peer platform for the next generation of financial technology.  
The decentralized nature of the Dash network allows for highly resilient Dash infrastructure,  
and the developer community needs reliable, open-source tools to implement Dash apps and services.    


## Getting Started

In order to use this library, you will need to add it to your project as a dependency.  

Having [NodeJS](https://nodejs.org/) installed, just type : `npm install @dashevo/wallet-lib` in your terminal.  
and add `const { Wallet } = require('@dashevo/wallet-lib');` on top of the file that will create and handle your wallet object.  

## Some rules of thumb

- There is multiple event listeners(socker sync,...), running intervals (service worker,...),  
therefore a good way to quit an instance would be to call `account.disconnect()` which will care to  
call `clearWorker(), closeSocket()` of the differents elements. You can still decide to remove them by hand if you want.  
- Some classic exemples of usage can be seen here : [Exemples](/docs/exemples.md)  

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
const { Mnemonic } = require('@dashevo/dashcore-mnemonic');
const localForage = require('localforage');


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
    mode: 'full', //BIP44 Worker (prederive addresses)
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

For more informations about the differents methods and options of the Account class, look at the [Wallet documentation](/docs/account.md).

### Pay to an address
```
const options = {
    to: "yizmJb63ygipuJaRgYtpWCV2erQodmaZt8",
    satoshis:100000,
    isInstantSend:false
};
const rawtx = account.createTransaction(options);
const txid = account.broadcastTransaction(rawtx);
```

## Features

BIPS Supports :

- [x] BIP32 (Hierarchical Deterministic Wallets)
- [ ] BIP38 (Passphrase protected private key)
- [x] BIP39 (Mnemonic code for generating deterministic keys)
- [x] BIP43 (Purpose field for deterministic wallets)
- [x] BIP44 (Multi-Account Hierarchy for Deterministic Wallets)

Miscellaneous :

- [ ] DAPI Support
- [ ] Protocol Support
- [x] Persistance interface
    - [x] In-mem support
    - [ ] DB support
- [x] HDWallet : Create a wallet / generate a new mnemonic
- [x] HDWallet : Validate mnemonic (BIP39)
- [x] HDWallet : Create an account
- [x] HDWallet : Address Derivation
- [x] HDWallet : Encrypt PrivKey (BIP38) - Passphrase Protection
- [x] HDWallet : Import (Mnemonic, HDPrivKey)
- [x] HDWallet : Export (Mnemonic, HDPrivKey)
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

Transports :

- [ ] DAPI-Client : [https://github.com/dashevo/dapi-client]
- [x] Insight- Client : [src/transports/Insight/insightClient.js]
- [ ] Dash-P2P : [src/transports/Protocol/protocolClient.js]
- [ ] DashcoreRPC : [src/transports/RPC/rpcClient.js]

Adapters (help from community welcomed) :

- [x] [LocalForage](https://github.com/localForage/localForage)
- [ ] LocalStorage
- [ ] SecureStorage
- [ ] LevelDB
- [ ] MongoDB
- [ ] BerkeleyDB
- [ ] LowDB

## API

- [Wallet](/docs/wallet.md)
- [Account](/docs/account.md)
- [KeyChain](/docs/keychain.md)
- [Storage](/docs/storage.md)
- [CoinSelection](/docs/coinSelection.md)

### Workers

- [BIP44Worker](/docs/BIP44Worker.md)
- [SyncWorker](/docs/SyncWorker.md)

## Examples

You can see here, some [Examples](/docs/examples.md).

## Road map

- DAPIClient : (09/18)
- DAPI Support (09/18)
- Improved coinSelection (10/18)
- Compatibility with old format and paper sweep wallet (10/18)
- Protocol Support (06/19)

## License

Wallet-lib is made available under the [MIT License](LICENSE)

## Credits

Wallet-Lib is maintained by the Dash Core Developers.
We want to thanks all member of the community that have submited suggestions, issues and pull requests.

