# Wallet workers

In order to perform it's duty of being in-sync with the network and to always keep a pre-generated set of unused addresses, wallet-lib uses internally two workers : 
- Sync Worker : Used to keep in sync with the network (utxo, received transactions,...)
- Chain Worker : Used to keep track of the current chain (best block height,...)
- BIP44 Worker : Used to always have a set of 20 unused addresses as per BIP44.

Theses default workers can be deactivated by adding the options `injectDefaultPlugins` to `false` while initializing your Wallet instance.


## Start a worker

```
worker.startWorker();
```

## Stop a worker

```
worker.stopWorker();
```

## Sync Worker

### Events

- WORKER/SYNC/STARTED - Triggered when the worker is started.
- WORKER/SYNC/EXECUTED - Triggered each time the worker get executed.

## BIP 44 Worker

### Create a BIP44 worker

```
const {events, storage, getAddress} = account;
const opts = {
 events,
 storage,
 getAddress
}
const worker = new BIP44Worker(opts);
```

### Events

- WORKER/BIP44/STARTED - Triggered when the worker is started.
- WORKER/BIP44/EXECUTED - Triggered each time the worker get executed.
