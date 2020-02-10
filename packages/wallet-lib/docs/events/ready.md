**Usage**: `account.events.on('ready', fn)`    
**Description**: Wallet-lib, when finished to perform it's internal tasks (blockheight, SPV, utxos sync...), will throw this event.

Example: 
```js
const {EVENTS} = require('@dashevo/wallet-lib');
const onReady = ()=>{
  console.log('Wallet-lib is ready to perform action');
}
account.events.on(EVENTS.READY, onReady);
```

