## Events 

```javascript
const {EVENTS} = require('@dashevo/wallet-lib');
const {FETCHED_CONFIRMED_TRANSACTION} = EVENTS;
const doSomethingConfirmedTransactionFetched = (tx) => {...}
account.on(FETCHED_CONFIRMED_TRANSACTION, doSomethingConfirmedTransactionFetched);
```

Events types : 


### Storage

| Event Name                 | Description                                             | 
| -------------------------- |:-------------------------------------------------------:|
| CONFIGURED                 |  throwed when Storage has configured the adapter.       |
| REHYDRATE_STATE_FAILED     | onFailedRehydrateState                                  |
| REHYDRATE_STATE_SUCCESS    | throwed when Storage has succesfully rehydrated the data|

### General 

| Event Name                 | Description                          | 
| -------------------------- |:------------------------------------:|
| READY                      |  throwed when ready to be used       |



### Sync Info

| Event Name                       | Description                                                          | 
| -------------------------------- |:--------------------------------------------------------------------:|
| BLOCKHEIGHT_CHANGED              | When the chain has moved from one block forward                      |
| FETCHED_UNCONFIRMED_TRANSACTION  | When we got to fetch an unconfirmed transaction, we throw this event |
| FETCHED_CONFIRMED_TRANSACTION    | This one is if the transaction is confirmed                          |
| FETCHED_TRANSACTIONS             | In both case, we throw that event                                    |


### Balance

| Event Name                   | Description                                                            | 
| ---------------------------- |:----------------------------------------------------------------------:|
| UNCONFIRMED_BALANCE_CHANGED  | When unconfirmed balance change, we gives the delta + totalValue       |
| BALANCE_CHANGED              | When the balance change, we gives the delta + totalValue               |


