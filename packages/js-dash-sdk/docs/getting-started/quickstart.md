# Quick start

In order to use this library, you will need to add our [NPM package](https://www.npmjs.com/dash) to your project.

Having [NodeJS](https://nodejs.org/) installed, just type :

```bash
npm install dash
## Initialize
```
## Initialization

Let's create a DashJS SDK instance specifying both our mnemonic and the schema we wish to work with.

```js
const DashJS = require("../src");
const opts = {
  network: 'testnet',
  apps: {
    dashpay: {
      contractId:1234,
      schema: require('schema.json')
    },
  },
  mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
};
const sdk = new DashJS.SDK(opts);
sdk.isReady().then(()=>{
    const activeAccount = sdk.account;
 });
```

Quick note :
- If no mnemonic is provided, the subinstance `sdk.Wallet` will not be initiated (write function for platforms won't be usable).

If you do not have any mnemonic, you can pass `null` to get one generated or omit that parameter to only use DashJS in `read-only`.  


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
  .documents.fetch('dashpay.profile', {name:'bob'})
  .then((profile)=> console.log(profile));
```
