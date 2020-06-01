**Usage**: `await wallet.getAccount([opts])`    
**Description**: This method will get you the account specified by it's index. 

Parameters: 

| parameters             | type      | required       | Description                                                                       |  
|------------------------|-----------|----------------| -------------------------------------------------------------------------------	  |
| **opts.index**         | number    | no (default: 0)| The [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki) index |

Returns : Account.

N.B: When `getAccount` is called on a never initialized account, you can pass-it any of [Account options](/docs/account/Account.md), and the wallet will initiate it (.createAccount) for you with those passed params and returns you the account.   
s

