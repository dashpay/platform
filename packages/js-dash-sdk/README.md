# DashJS

[![Package Version](https://img.shields.io/github/package-json/v/dashevo/dashjs.svg?&style=flat-square)](https://www.npmjs.org/package/@dashevo/dashjs)
[![Build Status](https://img.shields.io/travis/com/dashevo/dashjs.svg?branch=master&style=flat-square)](https://travis-ci.com/dashevo/dashjs)

> Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)


## Table of Contents

- [State](#state)
- [Principles](#principles)
- [Install](#install)
- [Usage](#usage)
    - [Platform](#platform)
    - [Wallet](#wallet)
    - [Primitives](#primitives)
        - [Transaction](#transaction)
        - [Address](#address)
        - [Block](#block)
        - [UnspentOutput](#unspentoutput)
        - [HDPublicKey](#hdpublickey)
        - [HDPrivateKey](#hdprivatekey)
        - [PublicKey](#publickey)
        - [PrivateKey](#privatekey)
        - [Mnemonic](#mnemonic)
        - [Network](#network)
- [License](#license)

## State

Under active development.

## Principles

Dash is a powerful new peer-to-peer platform for the next generation of financial technology. The decentralized nature of the Dash network allows for highly resilient Dash infrastructure, and the developer community needs reliable, open-source tools to implement Dash apps and services.

## Install

### ES5/ES6 via NPM

In order to use this library, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type : `npm install @dashevo/dashjs` in your terminal.

```sh
npm install @dashevo/dashjs
```


### CDN Standalone

For browser usage, you can also directly rely on unpkg : 

```
<script src="https://unpkg.com/dash"></script>
```

## Usage

```js
import DashJS from "@dashevo/dashjs"; 
//const DashJS = require('../build/index'); for es5
import schema from "./schema.json"; // If you want to interact with L2 (DPA)

const network = "testnet";
const opts = {
    network,
    mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
    schema
};
const sdk = new DashJS.SDK(opts);
const acc = sdk.wallet.getAccount();
async function sendPayment(){
    const tx = await acc.createTransaction({recipient:{address:'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h', amount:0.12}})
    console.log(tx)
}

async function readDocument() {
    const profile = await sdk.platform.fetchDocuments('profile',{},opts)
    console.log(profile);
}
```

Notes : 

- Omitting mnemonic will set the Wallet functionalities in offlineMode (for resources savings purposes) and set a default mnemonic.  
 You can use `sdk.wallet.exportWallet()` to get the randomly generated mnemonic.
- Omitting a schema will unset the Platform functionalities.

### Platform

Access the [Platform documentation on dashevo/dashcore-lib](/docs/Platform.md)

### Wallet

Access the [Wallet documentation on dashevo/dashcore-lib](/docs/Wallet.md)


### Primitives

In order to facilitate manipulations between the dependencies, DashJS provides the standardized Primitives as implemented in [Dashcore-lib](https://github.com/dashevo/dashcore-lib).


#### Transaction 

The Transaction primitive allows easy creation and manipulation of transactions. It also allows signing when provided with a privatekey.  
Supports fee control and input/output access (which allows to pass a specific script).  
```js
import { Transaction } from '@dashevo/dashjs';
const tx = new Transaction(txProps)
```

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)

#### Address

Standardized representation of a Dash Address. Address can be instantiated from a String, PrivateKey, PublicKey, HDPrivateKey or HdPublicKey.  
Pay-to-script-hash multi-signatures from an array of PublicKeys are also supported.  

```js
import { Address } from '@dashevo/dashjs';
```

Access the [Address documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/address.md)

#### Block

Given a hexadecimal string representation of the block as input, the Block class allows you to have a deserialized representation of a Block or it's header. It also allows to validate the transactions in the block against the header merkle root.  
Transactions of the block can also be explored by iterating over elements in array (`block.transactions`).

`import { Block } from '@dashevo/dashjs'`

Access the [Block documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/block.md)

#### UnspentOutput

Representation of an UnspentOutput (also called UTXO as in Unspent Transaction Output).  
Mostly useful in association with a Transaction and for Scripts. 

`import { UnspentOutput } from '@dashevo/dashjs'`

Access the [UnspentOutput documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/unspentoutput.md)

#### HDPublicKey

Hierarchical Deterministic (HD) version of the PublicKey.  
Used internally by Wallet-lib and for exchange between peers (DashPay)

`import { HDPublicKey } from '@dashevo/dashjs'`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/hierarchical.md)

#### HDPrivateKey

Hierarchical Deterministic (HD) version of the PrivateKey.  
Used internally by Wallet-lib.

`import { HDPrivateKey } from '@dashevo/dashjs'`

Access the [HDKeys documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/hierarchical.md)

#### PublicKey

`import { PublicKey } from '@dashevo/dashjs'`

Access the [PublicKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/publickey.md)

#### PrivateKey

`import { PrivateKey } from '@dashevo/dashjs'`

Access the [PrivateKey documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/privatekey.md)

#### Mnemonic

Implementation of [BIP39 Mnemonic code for generative deterministic keys](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki).  
Allow to generate random mnemonic on the language set needed, validate a mnemonic or get the HDPrivateKey associated.  

`import { Mnemonic } from '@dashevo/dashjs'`

Access the [Mnemonic documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/mnemonic.md)

#### Network

A representation of the internal parameters relative to the network used. By default, all primitives works with 'livenet', this class allow to have an testnet instance to used on the other primitives (such as Addresses), or for Wallet-lib.

`import { Network } from '@dashevo/dashjs'`


Access the [Network documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/networks.md)

#### Script

In Dash, transaction have in their inputs and outputs some script, very simple programming language with a stack-based evaluation and which is not Turing Complete.
A valid Transaction is a transaction which output script are evaluated as valid.  

Some operations of this language, such as OP_RETURN has been used to store hashes and B64 data on the payment chain.  
Learn more on our walkthrough [Transaction script manipulation with the OP_RETURN example](/docs/walkthroughs/op_return/or_return.md)

`import { Script } from '@dashevo/dashjs'`

Access the [Script documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/script.md)


#### Input

`import { Input } from '@dashevo/dashjs'`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)


#### Output

`import { Output } from '@dashevo/dashjs'`

Access the [Transaction documentation on dashevo/dashcore-lib](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md)


## Licence

[MIT](/LICENCE.md) © Dash Core Group, Inc.

