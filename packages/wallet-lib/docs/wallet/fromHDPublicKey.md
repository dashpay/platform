**Usage**: `wallet.fromHDPublicKey(HDPublicKey)`       
**Description**: Initialize a Wallet from a HDPublicKey representation.  
**Notes**: This is an internal method, in the future, when TC39 proposal pass, we will use the private markup.   

Parameters: 

| parameters             | type                   | required       | Description                                                         |  
|------------------------|------------------------|----------------| --------------------------------------------------------------------|
| **HDPublicKey**        | HDPublicKey|String     | yes            | The HDPublicKey from which you want to initialize the wallet.      |

Returns : void (set a HD Wallet in watch mode).

Examples : 

```js
wallet.fromHDPrivateKey('tpubDEB6BgW9JvZRWVbFmwwGuJ2vifakABuxQWdY9yXbFC2rc3zagie1RkhwUEnahb1dzaapchEVeKqKcx99TzkjNvjXcmoQkLJwsYnA1J5bGNj')
```


