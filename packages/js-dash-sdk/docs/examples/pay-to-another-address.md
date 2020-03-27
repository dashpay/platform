## Paying to another address

In order to pay, you need to have an [existing balance](/examples/receive-money-and-check-balance.md).   
The below code will allow you to pay to a single address a specific amount of satoshis.

```js
const Dash = require("dash");
const mnemonic = ''// your mnemonic here.
const client = new Dash.Client({
  mnemonic,
});

client.isReady().then(payToRecipient);

async function payToRecipient() {
    const {account} = client;
    const transaction = account.createTransaction({
      recipient:"yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf",
      satoshis:10000
    });
    const transactionId = await account.broadcastTransaction(transaction);
}
```

See more on create [transaction options here](https://dashevo.github.io/wallet-lib/#/usage/account?id=create-transaction).
