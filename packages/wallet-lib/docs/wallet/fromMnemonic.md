**Usage**: `wallet.fromMnemonic(mnemonic)`       
**Description**: Initialize a Wallet from a Mnemonic representation.  
**Notes**: This is an internal method, in the future, when TC39 proposal pass, we will use the private markup. Mnemonic initialized wallet works a little differently as they store the mnemonic in the wallet object.

Parameters: 

| parameters             | type                   | required       | Description                                                         |  
|------------------------|------------------------|----------------| --------------------------------------------------------------------|
| **mnemonic**           | Mnemonic|String        | yes            | The Mnemonic from which you want to initialize the wallet.      |

Returns : void (set a HD Wallet).

Examples : 

```js
wallet.fromMnemonic('knife easily prosper input concert merge prepare autumn pen blood glance toilet')
```


