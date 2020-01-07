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
    const {account} = sdk;
    // Do something
 });
```

Quick note :
- If no mnemonic is provided, the subinstance `sdk.Wallet` will not be initiated (write function for platforms won't be usable).

If you do not have any mnemonic, you can pass `null` to get one generated or omit that parameter to only use DashJS in `read-only`.  


## Make a payment

```js
sdk.isReady().then(()=>{
     const {account} = sdk;

    account
      .createTransaction({
        recipient:{address:'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h', amount:0.12}
      })
      .then(account.broadcastTransaction);
  });
```

## Read a document 

At time of writing, you will need to have registered dashpay yourself, see on [publishing a new contract](/examples/publishing-a-new-contract.md).

```js
sdk.isReady().then(async ()=>{
    const {account} = sdk;
    const bobProfile = await account.platform.documents.fetch('dashpay.profile', {name:'bob'})
  });
```
