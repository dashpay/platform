**Usage**: `wallet.fromSeed(seed)`       
**Description**: Initialize a Wallet from a seed.  
**Notes**: This is an internal method, in the future, when TC39 proposal pass, we will use the private markup.   
**Notes 2**: This actually transform seed in HDPrivateKey and uses `wallet.fromHDPrivateKey()`.  

Parameters: 

| parameters             | type      | required       | Description                                                         |  
|------------------------|-----------|----------------| --------------------------------------------------------------------|
| **seed**               | String    | yes            | The seed from which you want to initialize the wallet.              |

Returns : void (set a HD wallet).

Examples : 

```js
wallet.fromSeed('9e55e5cb0e2fe273600cf5af7d7760fe569121c320395f0233202b97445e54f577d5a706c49aa1f3f0993d2ff97e2e6d63e4ccd0b7e9d4c4f115ae58957a9114')
```


