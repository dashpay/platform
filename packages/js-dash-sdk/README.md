# Dash SDK

[![NPM Version](https://img.shields.io/npm/v/dash)](https://www.npmjs.org/package/dash)
[![Build Status](https://img.shields.io/travis/com/dashevo/js-dash-sdk)](https://travis-ci.com/dashevo/js-dash-sdk)
[![Release Date](https://img.shields.io/github/release-date/dashevo/js-dash-sdk)](https://github.com/dashevo/js-dash-sdk/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)

Dash library allows you to connect to DAPI and receive or broadcast payments on the Dash Network, manage identifies, register data contracts, retrieve or submit documents on the Dash Platform, all within a single library.

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

// Accessing an account allow you to transact with the Dash Network
client.getWalletAccount().then(async (account) => {
  console.log("Funding address", account.getUnusedAddress().address);

  const balance = account.getConfirmedBalance();
  console.log("Confirmed Balance", balance);

  if(balance > 0){
    // Creating an identity is the basis of all interactions with the Dash Platform
    const identity = await client.platform.identities.register()
    
    // Prepare a new document containing a simple hello world sent to a hypothetical tutorial contract
    const document = await platform.documents.create(
      'tutorialContract.note',
      identity,
      { message: 'Hello World' },
    );

    // Broadcast the document into a new state transition
    await platform.documents.broadcast({create:[document]}, identity);
  }
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

More extensive documentation available at https://dashevo.github.io/js-dash-sdk/.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/js-dash-sdk/issues/new/choose) or submit PRs.

## License

[MIT](/LICENSE) © Dash Core Group, Inc.
