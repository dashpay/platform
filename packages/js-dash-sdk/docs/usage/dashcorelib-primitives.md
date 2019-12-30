## Transaction 

The Transaction primitive allows easy creation and manipulation of transactions. It also allows signing when provided with a privatekey.  
Supports fee control and input/output access (which allows passing a specific script).
```js
import { Transaction } from 'dash';
const tx = new Transaction(txProps)
```

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)

## Address

Standardized representation of a Dash Address. Address can be instantiated from a String, PrivateKey, PublicKey, HDPrivateKey or HdPublicKey.  
Pay-to-script-hash (P2SH) multi-signature addresses from an array of PublicKeys are also supported.  

```js
import { Address } from 'dash';
```

Access the [Address documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/address.md)

## Block

Given a hexadecimal string representation of the block as input, the Block class allows you to have a deserialized representation of a Block or its header. It also allows validating the transactions in the block against the header merkle root.

Transactions of the block can also be explored by iterating over elements in array (`block.transactions`).  

`import { Block } from 'dash'`

Access the [Block documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/block.md)

## UnspentOutput

Representation of an UnspentOutput (also called UTXO as in Unspent Transaction Output).  
Mostly useful in association with a Transaction and for Scripts. 

`import { UnspentOutput } from 'dash'`

Access the [UnspentOutput documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/unspentoutput.md)

## HDPublicKey

Hierarchical Deterministic (HD) version of the PublicKey.  
Used internally by Wallet-lib and for exchange between peers (DashPay)

`import { HDPublicKey } from 'dash'`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/hierarchical.md)

## HDPrivateKey

Hierarchical Deterministic (HD) version of the PrivateKey.  
Used internally by Wallet-lib.

`import { HDPrivateKey } from 'dash'`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/hierarchical.md)

## PublicKey

`import { PublicKey } from 'dash'`

Access the [PublicKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/publickey.md)

## PrivateKey

`import { PrivateKey } from 'dash'`

Access the [PrivateKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/privatekey.md)

## Mnemonic

Implementation of [BIP39 Mnemonic code for generative deterministic keys](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki).  
Allow to generate random mnemonic on the language set needed, validate a mnemonic or get the HDPrivateKey associated.  

`import { Mnemonic } from 'dash'`

Access the [Mnemonic documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/mnemonic.md)

## Network

A representation of the internal parameters relative to the network used. By default, all primitives works with 'livenet', this class allow to have an testnet instance to used on the other primitives (such as Addresses), or for Wallet-lib.

`import { Network } from 'dash'`


Access the [Network documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/networks.md)

## Script

In Dash, transaction have in their inputs and outputs some script, very simple programming language with a stack-based evaluation and which is not Turing Complete.
A valid Transaction is a transaction which output script are evaluated as valid.  

Some operations of this language, such as OP_RETURN has been used to store hashes and B64 data on the payment chain.  
Learn more on our walkthrough [Transaction script manipulation with the OP_RETURN example](/docs/walkthroughs/op_return/or_return.md)

`import { Script } from 'dash'`

Access the [Script documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/script.md)


## Input

`import { Input } from 'dash'`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)


## Output

`import { Output } from 'dash'`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)
