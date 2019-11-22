# Quick start

In order to use this library, you will need to add our [NPM package](https://www.npmjs.com/@dashevo/dashjs) to your project.

Having [NodeJS](https://nodejs.org/) installed, just type :

```bash
npm install @dashevo/dashjs
## Initialize
```
## Initialization

Let's create a DashJS SDK instance specifying both our mnemonic and the schema we wish to work with.

```js
const DashJS = require("../src");
const opts = {
  network: 'testnet',
  schema: require('schema.json'),
  mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
};
const sdk = new DashJS.SDK(opts);
const activeAccount = sdk.wallet.getAccount();
```

Quick note :
- If no schema is provided, the subinstance `sdk.Platform` will not be initiated.
- If no mnemonic is provided, the subinstance `sdk.Wallet` will not be initiated.

## Make a payment

```js
activeAccount
  .createTransaction({
    recipient:{address:'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h', amount:0.12}
  })
  .then((tx)=> console.log(tx));
```

## Read a document

```js
activeAccount.platform
  .fetchDocuments('profile', {name:'bob'})
  .then((profile)=> console.log(profile));
```
