**Usage**: `account.events.on('FETCHED/UNCONFIRMED_TRANSACTION', fn)`    
**Description**: Every time a new unconfirmed transaction is fetched from the network, an event in thrown.

Returns : {Transaction}

Example: 
```js
const {EVENTS} = require('@dashevo/wallet-lib');
const onNewUnconfirmedTx = (tx)=>{
  console.log('Unconfirmed tx', tx);
}
account.events.on(EVENTS.FETCHED_UNCONFIRMED_TRANSACTION, onNewUnconfirmedTx);
```

