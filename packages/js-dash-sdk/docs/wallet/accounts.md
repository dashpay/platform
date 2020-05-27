## Getting an account

A Wallet is actually a holders of multiple Account that hold the keys needed to make a payment.  
So the first thing will be on accessing your account : 

```js
const client = new Dash.Client({
  wallet: {
    mnemonic: "maximum blast eight orchard waste wood gospel siren parent deer athlete impact",
  },
});
client.isReady().then(()=>{
  const {account} = client;
  // Do something with account
});
```

As optional parameter, an integer representing the account `index` can be passed as parameter. By default, index account on call is 0.

You will also see that we wait for isReady to resolve before making any operation. This allow us to access an account instance that will have synced-up with network and that our UTXOS set are ready to be used for payment/signing.  

