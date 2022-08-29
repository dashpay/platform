# Dashcore Lib primitives

All Dashcore lib primitives are exposed via the `Core` namespace.

```js
const Dash = require('dash');
const {
  Core: {
    Block,
    Transaction,
    Address,
    // ...
  }
} = Dash;
```

## Transaction 

The Transaction primitive allows creating and manipulating transactions. It also allows signing transactions with a private key.  
Supports fee control and input/output access (which allows passing a specific script).

```js
const { Transaction } = Dash.Core;
const tx = new Transaction(txProps)
```

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/transaction.md)

## Address

Standardized representation of a Dash Address. Address can be instantiated from a String, PrivateKey, PublicKey, HDPrivateKey or HdPublicKey.  
Pay-to-script-hash (P2SH) multi-signature addresses from an array of PublicKeys are also supported.  

```js
const { Address } = Dash.Core;
```

Access the [Address documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/address.md)

## Block

Given a binary representation of the block as input, the Block class allows you to have a deserialized representation of a Block or its header. It also allows validating the transactions in the block against the header merkle root.

The block's transactions can also be explored by iterating over elements in array (`block.transactions`).  

`const { Block } = Dash.Core;`

Access the [Block documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/block.md)

## UnspentOutput

Representation of an UnspentOutput (also called UTXO as in Unspent Transaction Output).  
Mostly useful in association with a Transaction and for Scripts. 

`const { UnspentOutput } = Dash.Core.Transaction;`

Access the [UnspentOutput documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/unspentoutput.md)

## HDPublicKey

Hierarchical Deterministic (HD) version of the PublicKey.  
Used internally by Wallet-lib and for exchange between peers (DashPay)

const { HDPublicKey } = Dash.Core;`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/hierarchical.md#hdpublickey)

## HDPrivateKey

Hierarchical Deterministic (HD) version of the PrivateKey.  
Used internally by Wallet-lib.

`const { HDPrivateKey } = Dash.Core;`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/hierarchical.md#hdprivatekey)

## PublicKey

`const { PublicKey } = Dash.Core;`

Access the [PublicKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/publickey.md)

## PrivateKey

`const { PrivateKey } = Dash.Core;`

Access the [PrivateKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/privatekey.md)

## Mnemonic

Implementation of [BIP39 Mnemonic code for generative deterministic keys](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki).  
Generates a random mnemonic with the chosen language, validates a mnemonic or returns the associated HDPrivateKey.  

`const { Mnemonic } = Dash.Core;`

Access the [Mnemonic documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/mnemonic.md)

## Network

A representation of the internal parameters relative to the selected network. By default, all primitives works with 'livenet'.

`const { Network } = Dash.Core;`


Access the [Network documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/networks.md)

## Script

`const { Script } = Dash.Core.Transaction;`

Access the [Script documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/script.md)


## Input

`const { Input } = Dash.Core.Transaction;`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/transaction.md#adding-inputs)


## Output

`const { Output } = Dash.Core.Transaction;`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/core-concepts/transaction.md#handling-outputs)
