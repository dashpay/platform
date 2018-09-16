## Account

### Get Addresses

```
const addresses = account.getAddresses();
const internalAddresses = account.getAddresses(false);
```


### Get Unused Address

`const address = account.getUnusedAddress()`

Additional parameters :
 - isExternal : Boolean - Default : `true`
 - skip : Integer - Default : `0`

### Get Private Keys

`const privateKeys = account.getPrivateKeys(addressList)`

Parameters :
- addressList : Array<String> - required

### Sign

`const signedTransaction = account.sign(transaction, privatekeys, sigtype)`

### Get Balance

Will return the balance amount in satoshis,

`const balance = account.getBalance()`


Additional parameters :
 - unconfirmed : Boolean - Default : `true` - Return the balance including unconfirmed inbound tx
 - displayDuffs : Boolean - Default : `true` - Return in either duff or dash

### Get UTXO

`const utxos = account.getUTXOS()`

Additional parameters :
 - onlyAvailable : Boolean - Default : `true`

### Disconnect

```
account.disconnect();
```
