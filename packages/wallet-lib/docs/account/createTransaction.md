**Usage**: `account.createTransaction(txOpts)`    
**Description**: Allow to create a transaction to one or multiple recipients.

Parameters: 

| parameters                    | type                          | required                     | Description                                                                                                                         |  
|-------------------------------|-------------------------------|------------------------------| ----------------------------------------------------------------------------------------------------------------------------------- |
| **txOpts.recipient**          | string                        | yes (if no `recipients`)     | The external address recipient of this transaction                                                                                  |
| **txOpts.satoshis**           | string                        | yes (if no `recipients` set) | The value amount to transfer to the recipient address                                                                               |
| **txOpts.recipients**         | Array[{recipient, satoshis}]  | no                           | Alternatively, you can use this to send to multiple address/amount. Array arra of {recipient, satoshis}                             |
| **txOpts.utxos**              | Array[utxos]                  | no                           | Can be specified to use specific utxo to use, or other utxos own by other private keys (you will need to pass the privateKeys along |
| **txOpts.privateKeys**        | Array[PrivateKey/HDPrivateKey]| no                           | Overwrite the default behaviour (searching locally for keys) and uses these to sign instead.                                        |
| **txOpts.strategy**           | string/Function               | no                           | Overwrite the default strategy used (using account default or specified strategy)                                                   |
| **txOpts.deductFee**          | boolean                       | no                           | Defaults: true. When set at false, will not deduct fee on the Transaction object                                                    |
| **txOpts.change**             | string                        | no                           | Defaults: `account.getUnusedAddress(internal)`. When set, will use that address as a change address on remaining fund               |


Returns : [Transaction](https://dashevo.github.io/DashJS/#/usage/dashcorelib-primitives?id=transaction)   
Notes: This transaction will be need to be signed [`account.sign(transaction)`](/account/sign) and then, if wanted, broadcasted to the network for execution `account.broadcastTransaction()`.

Example : 
```js
const recipients = [{recipient:"yereyozxENB9jbhqpbg1coE5c39ExqLSaG", satoshis:10e8},{recipient: "yMN2w8NiwcmY3zvJLeeBxpaExFV1aN23pg", satoshis: 1e8}];
const change = "yaVrJ5dgELFkYwv6AydDyGPAJQ5kTJXyAN";
const tx = account.createTransaction({recipients, change});
```

**Strategy:**

By default, wallet-lib is shipped with two different strategies : 

- **simpleDescendingStrategy** : Will maximize the use of big inputs to meet the amount required.  
    Allows the fee to be optimized for the smallest size at the cost of breaking big inputs.
- **simpleAscendingStrategy** : Will try to use as many small inputs as possible to meet the amount required.  
    Allows using many small inputs at the cost of a potentially bigger fee.

You can also pass your own strategy (as a function) to allow you to create your own strategy for how you will want to spend the UTXO.   

```js
const recipient = "yereyozxENB9jbhqpbg1coE5c39ExqLSaG";
const satoshis = 10e8;
const specialStrategy = (utxosList, outputsList, deductFee = false, feeCategory = 'normal')=> { 
//...
};
const txOpts1 = {
     recipient, satoshis,
     strategy: 'simpleAscendingStrategy',
};
const txOpts2 = {
     recipient, satoshis,
     strategy: specialStrategy,
};
const tx1 = account.createTransaction(txOpts1);
const tx2 = account.createTransaction(txOpts2);
```

See more information about [coinSelection](/usage/coinSelection).

## Deduct Fee 

In order to broadcast a transaction, a minimal relay fee is required for a node to accept to broadcast the transaction.  

Such fee are used as a spam mechanism protection as a standard transaction would require slightly more than 0.0000012 Dash (varies per transaction and per node) as relay fee.  

The deduct fee property, when set at true allows to automatically estimate the size and deduct from outputs the corresponding amount.  

In case one user would want to not see that, he will be required to select an input to pay a fee by himself. 

Expected minimal relay fee for your transaction can be estimated this way : 

```js 
const { storage, network } = account;
const { chains } = storage.getStore();
const txOpts = {
deductFee: false,
}
const transaction = account.createTransaction(txOpts);

const { minRelay: minRelayFeeRate } = chains[network.toString()].fees;

const estimateKbSize = transaction._estimateSize() / 1000;
const minFeeToPay = estimateKbSize * minRelayFeeRate;
```
