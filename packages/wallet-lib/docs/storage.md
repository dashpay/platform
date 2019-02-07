## Storage

The Storage class handle all primitives of a state of a wallet (Transactions, Addresses, Accounts,...).

It store them (using importTransaction/Address impacting when needed the storage areas (utxos of a specific address)) method, provide search utility function.


## Create a Storage Manager

```
const store = new Storage([opts]);
```

The Storage object will later on need to be configured before being ready for use.

##### options

> **rehydrate** : Bool(def: true) - If able, will rehydrate the state onLoad

> **autosave** : Bool(def: true) - If able, will autosave the state onChange

---

## Configure

`store.configure([opts])`

##### options

> **rehydrate** : Bool(def: herited) - If able, will rehydrate the state onLoad

> **autosave** : Bool(def: herited) - If able, will autosave the state onChange

> **adapter** : Adapter(def: defaultAdapter) - Allow to specifically set an adapter, by default will try to use localforage and defaults on InmemoryStorage

---

## getStore

Returns a snapshot (cloned copy) of the state of the store

`store.getStore()`

---

## getTransaction

`store.getTransaction(txid)`

##### params

> **txid** : TxId - A valid transaction id - required

---

## searchAddress

`store.searchAddress(address, [forceLoop])`

##### params

> **address** : Address - A valid Address

> **forceLoop** : Bool (def: false) - When set at true, will discard cache to do a full search

---

## searchTransaction

`store.searchTransaction(txid)`

##### params

> **txid** : TxId - A valid TxId

---


## addUTXOToAddress
Allow to add a specific UTXO to a specific address

`store.addUTXOToAddress(utxo, address)`

##### params

> **utxo** : Utxo - A valid Utxo

> **address** : Address - A valid Address

---
