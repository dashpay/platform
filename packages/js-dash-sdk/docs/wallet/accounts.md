## Getting an account

When Wallet is initialized with `mnemonic`, it holds multiple Accounts according to BIP44. 
Each Account holds the keys needed to make a payments from it.

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

