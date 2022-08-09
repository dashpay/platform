# Dash SDK

[![NPM Version](https://img.shields.io/npm/v/dash)](https://www.npmjs.org/package/dash)
[![Release Packages](https://github.com/dashevo/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashevo/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashevo/platform)](https://github.com/dashevo/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)

Dash library provides access via [DAPI](https://dashplatform.readme.io/docs/explanation-dapi) to use both the Dash Core network and Dash Platform on [supported networks](https://github.com/dashevo/platform/#supported-networks). The Dash Core network can be used to broadcast and receive payments. Dash Platform can be used to manage identities, register data contracts for applications, and submit or retrieve application data via documents.

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
const Dash = require("dash"); // or import Dash from "dash"

const client = new Dash.Client({
  wallet: {
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
  },
  apps: {
    tutorialContract: {
      // Learn more on how to register Data Contract
      // https://dashplatform.readme.io/docs/tutorial-register-a-data-contract#registering-the-data-contract
      contractId: "<tutorial-contract-id>" 
    }
  }
});

// Accessing an account allow you to transact with the Dash Network
client.wallet.getAccount().then(async (account) => {
  console.log('Funding address', account.getUnusedAddress().address);

  const balance = account.getConfirmedBalance();
  console.log('Confirmed Balance', balance);

  if (balance > 0) {
    // Obtain identity - the base of all platform interactions
    // Read more on how to create an identity here: https://dashplatform.readme.io/docs/tutorial-register-an-identity
    const identityIds = account.identities.getIdentityIds();
    const identity = await client.platform.identities.get(identityIds[0]);

    // Prepare a new document containing a simple hello world sent to a hypothetical tutorial contract
    const document = await client.platform.documents.create(
      'tutorialContract.note',
      identity,
      { message: 'Hello World' },
    );

    // Broadcast the document into a new state transition
    await client.platform.documents.broadcast({ create: [document] }, identity);

    // Retrieve documents
    const documents = await client.platform.documents.get('tutorialContract.note', {
      limit: 2,
    });

    console.log(documents);
  }
});
```

### Primitives and essentials
Dash SDK bundled into a standalone package, 
so that the end user never have to worry about mananaging polyfills or related dependencies 

```javascript
const Dash = require('dash')

const {
  Essentials: {
    Buffer  // Node.JS Buffer polyfill.
  },
  Core: { // @dashevo/dashcore-lib essentials
    Transaction, 
    PrivateKey,
    BlockHeader,
    // ...
  },
  PlatformProtocol: { // @dashevo/dpp essentials
    Identity,
    Identifier,
  },
  WalletLib: { // @dashevo/wallet-lib essentials
    EVENTS
  },
  DAPIClient, // @dashevo/dapi-client
} = Dash;
``` 

## Dependencies 

The Dash SDK works using multiple dependencies that might interest you:
- [Wallet-Lib](https://github.com/dashevo/platform/tree/master/packages/wallet-lib) - Wallet management for handling, signing and broadcasting transactions (BIP-44 HD).
- [Dashcore-Lib](https://github.com/dashevo/dashcore-lib) - Provides the main L1 blockchain primitives (Block, Transaction,...).
- [DAPI-Client](https://github.com/dashevo/platform/tree/master/packages/js-dapi-client) - Client library for accessing DAPI endpoints.
- [DPP](https://github.com/dashevo/platform/tree/master/packages/js-dpp) - Implementation (JS) of Dash Platform Protocol.

Some features might be more extensive in those libs, as Dash SDK only wraps around them to provide a single interface that is easy to use (and thus has less features).

## Documentation

More extensive documentation available at https://dashevo.github.io/platform/SDK/.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/platform/issues/new/choose) or submit PRs.

## License

[MIT](/LICENSE) © Dash Core Group, Inc.
