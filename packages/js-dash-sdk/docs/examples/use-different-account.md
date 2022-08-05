## Using a different account 

Clients initialized with a mnemonic support multiple accounts as defined in [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki).

By default `client.wallet.getAccount()` returns the account at index `0`.

To access other accounts, pass the `index` option:
```
const secondAccount = await client.wallet.getAccount({ index: 1 })
``` 

