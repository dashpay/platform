## Getting an account

Wallet initialized with `mnemonic` holds multiple Accounts according to BIP44. 
Account holding the keys needed to make a payment.

Wallet's `getAccount` method used to access account

```js
const client = new Dash.Client({
  wallet: {
    mnemonic: "maximum blast eight orchard waste wood gospel siren parent deer athlete impact",
  },
});

const account = await client.wallet.getAccount()
// Do something with account
```

As optional parameter, an integer representing the account `index` can be passed as parameter. By default, index account on call is 0.
```
client.wallet.getAccount({ index: 1 })
```

Awaiting for `getAccount()` promise is needed to have wallet synced-up with network and making sure that UTXOS set is ready to be used for payment/signing.  

