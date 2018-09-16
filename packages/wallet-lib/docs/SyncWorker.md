## Sync Worker

### Create a worker

```
const {events, storage, transport, fetchAddressInfo, fetchTransactionInfo} = account;
const opts = {
 events,
 storage,
 transport,
 fetchAddressInfo,
 fetchTransactionInfo,
}
const worker = new BIP44Worker(opts);
```

### Start a worker

```
worker.startWorker();
```

### Stop a worker

```
worker.stopWorker();
```

### Events

- WORKER/SYNC/STARTED - Triggered when the worker is started.
- WORKER/SYNC/EXECUTED - Triggered each time the worker get executed.