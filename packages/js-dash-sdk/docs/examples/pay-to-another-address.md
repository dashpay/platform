## Paying to another address

In order to pay, you need to have an [existing balance](../examples/receive-money-and-check-balance.md).   
The below code will allow you to pay to a single address a specific amount of satoshis.

```js
const Dash = require('dash');

const mnemonic = ''; // your mnemonic here.
// Synchronization will take a lot of time with this configuration. 
// We can use `unsafeOptions.skipSynchronizationBeforeHeigth` option
// in combination with progress events as described here
// https://github.com/dashpay/platform/tree/v0.25-dev/packages/wallet-lib#usage

const client = new Dash.Client({
  wallet: {
    mnemonic,
  },
});

async function payToRecipient(account) {
  const transaction = account.createTransaction({
    recipient: 'yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf',
    satoshis: 10000,
  });
  const transactionId = await account.broadcastTransaction(transaction);
}

client.wallet.getAccount().then(payToRecipient);

```

See more on create [transaction options here](https://dashpay.github.io/platform/Wallet-library/account/createTransaction/).
