**Usage**: `storage.calculateDuffBalance(walletId, accountIndex, type)`    
**Description**: Perform a full calculation of the balance of a wallet and account set.   

Parameters: 

| parameters             | type                                      | required         | Description                                                             |  
|------------------------|-------------------------------------------|------------------| ------------------------------------------------------------------------|
| **walletId**           | String                                    |  yes             | The wallet identifier in which we the account is                        |
| **accountIndex**       | Number                                    |  yes             | The account index from which we want to perform the calculation         |
| **type**               | Enum['total', 'confirmed', 'unconfirmed'] |  no (def: total) | Depending of UTXO status, will calculate accordingly                                                        |


Returns: Number (duff - aka satoshis - value of the balance).  

