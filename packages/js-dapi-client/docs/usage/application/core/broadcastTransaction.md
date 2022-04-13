**Usage**: `await client.core.broadcastTransaction(transaction)`  
**Description**: Allow to broadcast a valid **signed** transaction to the network.

Parameters:

| parameters                | type                | required       | Description                                                                                      |
|---------------------------|---------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **transaction**           | Buffer              | yes            | A valid Buffer representation of a transaction                                                   |
| **options**               | Object              |                |                                                  |
| **options.allowHighFees** | Boolean             | no[=false]     | As safety measure, "absurd" fees are rejected when considered to high. This allow to overwrite that comportement |
| **options.bypassLimits**  | Boolean             | no[=false]     | Allow to bypass default transaction policy rules limitation |

Returns : transactionId (string).

N.B : The TransactionID provided is subject to [transaction malleability](https://dashcore.readme.io/docs/core-guide-transactions-transaction-malleability), and is not a source of truth (the transaction might be included in a block with a different txid).
