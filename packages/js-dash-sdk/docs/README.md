# Dash SDK

[![NPM Version](https://img.shields.io/npm/v/dash)](https://www.npmjs.org/package/dash)
[![Build Status](https://img.shields.io/travis/com/dashevo/js-dash-sdk)](https://travis-ci.com/dashevo/js-dash-sdk)
[![Release Date](https://img.shields.io/github/release-date/dashevo/js-dash-sdk)](https://github.com/dashevo/js-dash-sdk/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)

Dash library allows you to connect to DAPI and receive or broadcast payments on the Dash Network, manage identifies, register data contracts, retrieve or submit documents on the Dash Platform, all within a single library.

## Install

### Browser

```html
<script src="https://unpkg.com/dash"></script>
```

### Node

In order to use this library, you will need to add our [NPM package](https://www.npmjs.com/dash) to your project.

Having [NodeJS](https://nodejs.org/) installed, just type :

```bash
npm install dash
```

## Usage

```js
const Dash = require('dash');

const client = new Dash.Client({
  network: 'evonet',
  wallet: {
    mnemonic: 'arena light cheap control apple buffalo indicate rare motor valid accident isolate',
  },
});

// Accessing an account allows you to transact with the Dash Network
client.getWalletAccount().then(async (account) => {
  console.log('Funding address', account.getUnusedAddress().address);

  const balance = account.getConfirmedBalance();
  console.log('Confirmed Balance', balance);

  if (balance > 0) {
    // Creating an identity is the basis of all interactions with the Dash Platform
    const identity = await client.platform.identities.register();

    // Prepare a new document containing a simple hello world sent to a hypothetical tutorial contract
    const document = await client.platform.documents.create(
      'tutorialContract.note',
      identity,
      { message: 'Hello World' },
    );

    // Broadcast the document into a new state transition
    await client.platform.documents.broadcast({ create: [document] }, identity);
  }
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
