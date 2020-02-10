**Usage**: `wallet.generateNewWalletId()`      
**Description**: Internally, each wallet has a WalletId attached. This tries to be deterministic by actually just be a substring of a double sha256 hash from the input.   
**Notes**: This is an internal method, in the future, when TC39 proposal pass, we will use the private markup. Also, mutates Wallet.walletId.  

Parameters: 

| parameters             | type      | required       | Description                                                                       |  
|------------------------|-----------|----------------| -------------------------------------------------------------------------------	  |

Returns : String (wallet.walletId).

