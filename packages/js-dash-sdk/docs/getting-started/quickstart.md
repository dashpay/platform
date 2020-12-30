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
  apps: {
    dashpay: {
      contractId: '77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3',
    },
  },
  wallet: {
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
  },
};
const client = new Dash.Client(opts);
client.getWalletAccount().then(async (account) => {
  // Do something
})
```

Quick note :

- If no mnemonic is provided, the sub-instance `client.Wallet` will not be initialized (writing  capabilities of Dash Platform won't be usable).

If you do not have a mnemonic, you can pass `null` to have one generated or omit that parameter to only use Dash.Client for `read-only` operations.

## Make a payment

```js
client.getWalletAccount().then(async (account) => {
  const transaction = account.createTransaction({
    recipient: 'yixnmigzC236WmTXp9SBZ42csyp9By6Hw8',
    amount: 0.12,
  });
  account.broadcastTransaction(transaction);
});
```

## Read a document

At the time of writing, you will need to have registered a data contract yourself. See [publishing a new contract](/examples/publishing-a-new-contract.md).

```js

client.platform.documents.get(
  'tutorialContract.note',
  { limit: 1 }, // Only retrieve 1 document
).then(async (documents) => {
  console.log(documents);
});
```
