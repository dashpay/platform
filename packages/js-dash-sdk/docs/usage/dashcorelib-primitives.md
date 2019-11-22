## Transaction 

The Transaction primitives allow easy creation and manipulation of transaction. It also allow signing when provided a privatekey.  
Fee control and input/output access provided (which allow to pass along specific script).  
```js
import { Transaction } from '@dashevo/dashjs';
const tx = new Transaction(txProps)
```

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)

## Address

Standardized representation of a Dash Address. Address can be instantiated from a String, PrivateKey, PublicKey, HDPrivateKey or HdPublicKey.  
It also support pay-to-script-hash multi-sinatures from an array of PublicKeys.  

```js
import { Address } from '@dashevo/dashjs';
```

Access the [Address documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/address.md)

## Block

Given an hexadecimal string representation of the block as input, the Block class allow you to have a deserialized representation of a Block, it's header and can validate it.  
Transaction of the block can also be explorated by iterating on transactions member (`block.transactions`).

`import { Block } from '@dashevo/dashjs'`

Access the [Block documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/block.md)

## UnspentOutput

Representation of an UnspentOutput (also called UTXO as in Unspent Transaction Output).  
Mostly useful in association with a Transaction and for Scripts. 

`import { UnspentOutput } from '@dashevo/dashjs'`

Access the [UnspentOutput documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/unspentoutput.md)

## HDPublicKey

Hierarchical Deterministic (HD) version of the PublicKey.  
Used internally by Wallet-lib and for exchange between peers (DashPay)

`import { HDPublicKey } from '@dashevo/dashjs'`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/hierarchical.md)

## HDPrivateKey

Hierarchical Deterministic (HD) version of the PrivateKey.  
Used internally by Wallet-lib.

`import { HDPrivateKey } from '@dashevo/dashjs'`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/hierarchical.md)

## PublicKey

`import { PublicKey } from '@dashevo/dashjs'`

Access the [PublicKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/publickey.md)

## PrivateKey

`import { PrivateKey } from '@dashevo/dashjs'`

Access the [PrivateKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/privatekey.md)

## Mnemonic

Implementation of [BIP39 Mnemonic code for generative deterministic keys](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki).  
Allow to generate random mnemonic on the language set needed, validate a mnemonic or get the HDPrivateKey associated.  

`import { Mnemonic } from '@dashevo/dashjs'`

Access the [Mnemonic documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/mnemonic.md)

## Network

A representation of the internal parameters relative to the network used. By default, all primitives works with 'livenet', this class allow to have an testnet instance to used on the other primitives (such as Addresses), or for Wallet-lib.

`import { Network } from '@dashevo/dashjs'`


Access the [Network documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/networks.md)

## Script

In Dash, transaction have in their inputs and outputs some script, very simple programming language with a stack-based evaluation and which is not Turing Complete.
A valid Transaction is a transaction which output script are evaluated as valid.  

Some operations of this language, such as OP_RETURN has been used to store hashes and B64 data on the payment chain.  
Learn more on our walkthrough [Transaction script manipulation with the OP_RETURN example](/docs/walkthroughs/op_return/or_return.md)

`import { Script } from '@dashevo/dashjs'`

Access the [Script documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/script.md)


## Input

`import { Input } from '@dashevo/dashjs'`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)


## Output

`import { Output } from '@dashevo/dashjs'`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)
