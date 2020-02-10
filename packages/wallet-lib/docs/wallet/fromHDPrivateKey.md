**Usage**: `wallet.fromHDPrivateKey(HDPrivateKey)`       
**Description**: Initialize a Wallet from a HDPrivateKey representation.  
**Notes**: This is an internal method, in the future, when TC39 proposal pass, we will use the private markup.   

Parameters: 

| parameters             | type                   | required       | Description                                                         |  
|------------------------|------------------------|----------------| --------------------------------------------------------------------|
| **HDPrivateKey**       | HDPrivateKey|String    | yes            | The HDPrivateKey from which you want to initialize the wallet.      |

Returns : void (set a HD Wallet).

Examples : 

```js
wallet.fromHDPrivateKey('xprv9s21ZrQH143K37d2j9YW7snYGbAJJX9vzEZRwU7QEc4yP39t1Yc7t2Aw79aBBQWfLNqnpo9bFRnWoDv7xCPyBpLFHZvvrtVYfRv2zEBtnT5')
```


