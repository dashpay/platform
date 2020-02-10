**Usage**: `storage.createChain(network)`    
**Description**: Create, if not already existing, a chain in the store.       
**Notes**: This is an internal advanced function called on the creation of a Wallet. Also, at current state, both testnet and evonet uses the same "Testnet" object. Which might cause support issue when using both chain.       

Parameters: 

| parameters             | type              | required         | Description                                                             |  
|------------------------|-------------------|------------------| ------------------------------------------------------------------------|
| **network**            | Network/String    | yes              | The network of the chain to create                                              |


Returns: Boolean

