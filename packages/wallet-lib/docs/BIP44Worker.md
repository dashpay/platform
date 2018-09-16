## BIP 44 Worker

### Create a worker

```
const {events, storage, getAddress} = account;
const opts = {
 events,
 storage,
 getAddress
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

- WORKER/BIP44/STARTED - Triggered when the worker is started.
- WORKER/BIP44/EXECUTED - Triggered each time the worker get executed.