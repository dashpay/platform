**Usage**: async `storage.rehydrateState()`     
**Description**: Used to fetch the state from the persistence adapter    
**Notes**: Three items are fetch (`adapter.getItem`) : transactions, wallets and chains data.   

Parameters: 

| parameters             | type              | required       | Description                                                            |  
|------------------------|-------------------|----------------| -----------------------------------------------------------------------|


Returns: Void   
Emit: `REHYDRATE_STATE_SUCCESS/REHYDRATE_STATE_FAILED` event
