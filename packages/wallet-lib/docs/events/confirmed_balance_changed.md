**Usage**: `account.events.on('confirmed_balance_changed', fn)`    
**Description**: Wallet-lib, when finished to perform it's internal tasks (blockheight, SPV, utxos sync...), will throw this event.
**Important**: Standardization on event might happen soon, to avoid breaking change, use the EVENTS constant as described below. 

Example: 
```js
const {EVENTS} = require('@dashevo/wallet-lib');
const onConfirmedBalanceChange = ()=>{
  console.log('Balance changed');
}
account.events.on(EVENTS.CONFIRMED_BALANCE_CHANGED, onConfirmedBalanceChange);
```

