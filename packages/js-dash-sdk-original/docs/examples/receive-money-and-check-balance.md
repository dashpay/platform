## Receive money and display balance

Initialize the SDK Client with your [generated mnemonic](../examples/generate-a-new-mnemonic.md) passed as an option.

```js
const Dash = require("dash");
const mnemonic = ''// your mnemonic here.
const client = new Dash.Client({
  wallet: {
    mnemonic,
  }
});

async function showBalance() {
  const account = await client.wallet.getAccount();
  const totalBalance = account.getTotalBalance();
  console.log(`Account's total balance: ${totalBalance} duffs`);
}
```

Having your `client` instance set up, you will be able to access the `account` and `wallet` instance generated from your mnemonic.

By default `getAccount()` returns the first BIP44 account. 
You can read more on [how to use a different account](../examples/use-different-account.md).


## Generate a receiving address

Dash wallet supports two different types of addresses: 
- `external` addresses used for receiving funds from other addresses
- `internal` addresses used for change outputs of outgoing transactions  
- 
For your privacy, you might want to generate a new address for each payment:

```js
async function generateUnusedAddress() {
  const account = await client.wallet.getAccount();
  const { address } = account.getUnusedAddress();
  console.log(`Unused external address: ${address}`);
}
```

This above code will generate a new unique (never used) address. 

## Displaying your balance

_Dash Wallet returns the balance in duffs (1 Dash is equal to 100.000.000 duffs)_

`getTotalBalance()` function takes into account `confirmed` and `unconfirmed` transactions (not included in a block).
It is recommended to check the confirmed balance before making a payment:

```js
async function showBalance() {
  const account = await client.wallet.getAccount();
  const totalBalance = account.getTotalBalance();
  const confirmedBalance = account.getConfirmedBalance();
  const unconfirmedBalance = account.getUnconfirmedBalance();
  console.log(`Account balance:
    Confirmed: ${confirmedBalance}
    Unconfirmed: ${unconfirmedBalance}
    Total: ${totalBalance}
  `);
}
```
 

## Listen for event on received transaction 

When a new unconfirmed transaction is received, you can receive an event, and then validate the address or perform an action if needed.   

```js
// FETCHED/UNCONFIRMED_TRANSACTION event is currently disabled

async function listenUnconfirmedTransaction() {
  const account = await client.wallet.getAccount();
  account.on('FETCHED/UNCONFIRMED_TRANSACTION', (data) => {
    console.dir(data);
  });
}
```

## Get address at specific index 

In case you want to retrieve an address at specific index: 

```js
async function getAddressAtIndex() {
  const account = await client.wallet.getAccount();
  const { address: externalAddress } = account.getAddress(2);
  const { address: internalAddress } = account.getAddress(2, 'internal');
}
```
