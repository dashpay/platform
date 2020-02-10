**Usage**: `wallet.exportWallet([outputType])`    
**Description**: This method will export the wallet to the default outputType (depending on initializated params : mnemonic. HDPubKey,...). 

This method varies depending from which type of wallet is this. 
- When init from a mnemonic, by default return mnemonic but support 'HDPrivateKey'
- When init from a seed, by default returns and only support HDPrivateKey
- When init from a private key, by default returns and only support PrivateKey.

Parameters: 

| parameters             | type      | required       | Description                                                                       |  
|------------------------|-----------|----------------| ----------------------------------------------------------  |
| **outputType**         | String    | no             | The required output type of the exported wallet             |

Returns : {Mnemonic|HDPrivateKey|HDPublicKey|PrivateKey}


