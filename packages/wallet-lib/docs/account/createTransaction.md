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
| **txOpts.strategy**           | string                        | no                           | Overwrite the default strategy used (using account default or specified strategy)                                                   |
| **txOpts.deductFee**          | boolean                       | no                           | Defaults: true. When set at false, will not deduct fee on the Transaction object                                                    |
| **txOpts.change**             | string                        | no                           | Defaults: `account.getUnusedAddress(internal)`. When set, will use that address as a change address on remaining fund               |


Returns : [Transaction](https://dashevo.github.io/DashJS/#/usage/dashcorelib-primitives?id=transaction)   
Notes: This transaction will be need to be signed [`account.sign(transaction)`](/account/sign) and then, if wanted, broadcasted to the network for execution `account.broadcastTransaction()`.

Example : 
```js
const recipient = "yereyozxENB9jbhqpbg1coE5c39ExqLSaG";
const satoshis = 10e8;
const specialStrategy = (utxosList, outputsList, deductFee = false, feeCategory = 'normal')=> { 
//...
};
const txOpts1 = {
     recipient, satoshis,
     strategy: specialStrategy,
};
const tx1 = account.createTransaction(txOpts1);
```

```js
const recipients = [{recipient:"yereyozxENB9jbhqpbg1coE5c39ExqLSaG", satoshis:10e8},{recipient: "yMN2w8NiwcmY3zvJLeeBxpaExFV1aN23pg", satoshis: 1e8}];
const change = "yaVrJ5dgELFkYwv6AydDyGPAJQ5kTJXyAN";
const tx = account.createTransaction({recipients, change});
```

Deprecated : 
- opts.amount: will be removed in next breaking change release.
- opts.isInstantSend : Will be removed.

