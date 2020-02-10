**Usage**: `wallet.fromPrivateKey(privateKey)`       
**Description**: Initialize a Wallet from a PrivateKey representation.  
**Notes**: This is an internal method, in the future, when TC39 proposal pass, we will use the private markup.   

Parameters: 

| parameters             | type                   | required       | Description                                                         |  
|------------------------|------------------------|----------------| --------------------------------------------------------------------|
| **PrivateKey**         | PrivateKey|String      | yes            | The PrivateKey from which you want to initialize the wallet.        |

Returns : void (set a single address wallet).

Examples : 

```js
wallet.fromPrivateKey('cR4t6evwVZoCp1JsLk4wURK4UmBCZzZotNzn9T1mhBT19SH9JtNt')
```


