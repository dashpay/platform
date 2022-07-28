# Quick start

In order to use this library, you will need to add our [NPM package](https://www.npmjs.com/dash) to your project.

Having [NodeJS](https://nodejs.org/) installed, just type :

```bash
npm install dash
```

## Initialization

Let's create a Dash SDK client instance specifying both our mnemonic and the schema we wish to work with.

```js
const Dash = require('dash');
const opts = {
  wallet: {
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
  },
};
const client = new Dash.Client(opts);
client.wallet.getAccount().then(async (account) => {
  // Do something
})
```

Quick note:
If no `mnemonic` is provided or `mnemonic: null` is passed inside the `wallet` option, a new mnemonic will be generated.


## Make a payment

```js
client.wallet.getAccount().then(async (account) => {
  const transaction = account.createTransaction({
    recipient: 'yixnmigzC236WmTXp9SBZ42csyp9By6Hw8',
    amount: 0.12,
  });
  await account.broadcastTransaction(transaction);
});
```

## Interact with Dash Platform

See the [Tutorial section](https://dashplatform.readme.io/docs/tutorials-introduction) of the Dash Platform documentation for examples.
