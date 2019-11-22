## Getting an account

A Wallet is actually a holders of multiple Account that hold the keys needed to make a payment.  
So the first thing will be on accessing your account : 

```js
const account = wallet.getAccount();
account.events.on('ready', ()=>console.log(`I'm ready to create stuff!`));
```

As optional parameter, an integer representing the `accountIndex` can be passed as parameter. By default, index account on call is 0.

You will also see that we listen to the event `ready` before making any operation as it is the event we want to listen to tell us that everything is verified and our UTXOS set is ready to be used for payment/signing.  

