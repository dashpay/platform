**Usage**: `account.events.on('blockheight_changed', fn)`    
**Description**: An event is thrown each time Wallet-lib is being made aware of a new block validated by the protocol.

Example: 
```js
const {EVENTS} = require('@dashevo/wallet-lib');

account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, ({payload: blockHeight})=>{
  console.log(`Blockheight changed to ${blockHeight}`);
});
```

