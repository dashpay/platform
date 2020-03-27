## Receive money and display balance

Initialize the SDK Client with your [generated mnemonic](/examples/generate-a-new-mnemonic) passed as an option.  
By default, the SDK Client will work on Evonet, the only network having DAPI at the time of writing.

```js
const Dash = require("dash");
const mnemonic = ''// your mnemonic here.
const client = new Dash.Client({
  mnemonic,
});
```

Having set up your `client` instance, you be able to access the `account` and `wallet` instance generated from your mnemonic.

You can read more on [change my account](/examples/change-my-account) as by default, you are on the first BIP44 account. 


## Generate a receiving address

In a client, you have two different type of payment address, `external`, which are those used to receive you money from outside.   
And `internal`, which is used internally for the chance of a payment you do to someone.  
For your privacy, you might want to generate a new address at each payment.

```js
   client.isReady().then(generateNewAddress);
   async function generateNewAddress(){
     const {address} = client.account.getUnusedAddress();
     console.log(`New address: ${address}`)
   }
```

This above code will generate a new unique (never used) address. 

## Display your balance

There are three different balances, the `getTotalBalance()` that gives you the sum of `confirmed` and `unconfirmed` transactions (not included in a block). ```
You probably most of the time want to rely of your confirmed balance when you check fund before a payment.  
Value is in satoshis (smallest unit).

```js
   client.isReady().then(displayBalance);
   async function displayBalance(){
     const balance = client.account.getConfirmedBalance();
     console.log(`Balance: ${balance}`)
   }
```

Or you might want to look-up for the `.getUnconfirmedBalance()` to have only the unconfirmed amount. 

## Listen for event on received transaction 

When a new unconfirmed transaction is received, you can receive notice of it, to perform a validation on the address and perform an action if needed.   

```js
  client.account.events.on('FETCHED/UNCONFIRMED_TRANSACTION', (data)=>{
    console.log('FETCHED/UNCONFIRMED_TRANSACTION');
    console.dir(data)
  });
```

The return element can be used in coordination with a `new Dash.Core.Transaction()`

## Get an address 

In case you want to retrieve a specific address index : 

```js
  const {address} = client.account.getAddress(2);
```

## Get an internal address 

```js
  const {address} = client.account.getAddress(2, 'internal');
```
