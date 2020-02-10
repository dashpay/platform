**Usage**: async `storage.updateAddress(addressObj, walletId)`    
**Description**: Used to update a specific address of a wallet identified by it's walletId.

Parameters: 

| parameters             | type              | required       | Description                                               |  
|------------------------|-------------------|----------------| ----------------------------------------------------------|
| **addressObj**         | AddressObject     | yes            | The AddressObject to update (uses address.path as primary key)     |
| **walletId**           | String            | yes            | The Wallet identifier of the wallet containing the address to update     |

Returns : Boolean.

Example: 
```js
storage.updateAddress({
    path: "m/44'/1'/0'/0/0",
    index: '0',
    address: 'yLhsYLXW5sFHLDPLj2EHgrmQRhP712ANda',
    transactions: [],
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
    utxos: {},
    fetchedLast: 0,
    used: true,
  }, "a3771aaf93");
```

