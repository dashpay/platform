**Usage**: `storage.createWallet(walletId, network, mnemonic, type)`    
**Description**: Create a wallet in store based on the specified params.      
**Notes**: This is an internal advanced function called on the creation of a Wallet.    

Parameters: 

| parameters             | type              | required         | Description                                                             |  
|------------------------|-------------------|------------------| ------------------------------------------------------------------------|
| **walletId**           | String            | yes              | The wallet id to create                                                 |
| **network**            | Network/String    | no (Def: evonet) | The network for the wallet                                              |
| **mnemonic**           | Mnemonic/String   | no (Def: null)   | When applicable, the mnemonic used to generate the wallet               |
| **type**               | String            | no (Def: null)   | The wallet type to create                                               |


Returns: Boolean

