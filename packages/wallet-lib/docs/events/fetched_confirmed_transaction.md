**Usage**: `account.events.on('FETCHED/CONFIRMED_TRANSACTION', fn)`    
**Description**: Every time a new confirmed transaction is fetched from the network, an event in thrown.

Returns : {Transaction}

Example: 
```js
const {EVENTS} = require('@dashevo/wallet-lib');
const onNewConfirmedTx = (tx)=>{
  console.log('Confirmed tx', tx);
}
account.events.on(EVENTS.FETCHED_CONFIRMED_TRANSACTION, onNewConfirmedTx);
```
