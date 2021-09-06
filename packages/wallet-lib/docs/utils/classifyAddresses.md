**Usage**: `classifyAddresses(addressStore, accountIndex, walletType)`    
**Description**: Return for an addressStore, accountIndex and wallet type, the array classified set of addresses

Parameters: 

| parameters        | type          | required       | Description                                      |  
|-------------------|---------------|----------------| -------------------------------------------------|
| **addressStore**  | Object   | yes            | Account store with addresses                           |
| **accountIndex**  | Number   | yes            | The account index                           |
| **walletType**    | WALLET_TYPES   | yes            | The wallet type                           |

Returns : {[ClassifiedAddresses]} - Array of classified addresses 

```js
const classifiedAddresses = classifyAddresses(addressStore, 0, WALLET_TYPES.HDWALLET);

{
    externalAddressList: [
    'yd1ohc12LgCYp56CDuckTEHwoa6LbPghMd',
    '...'
    ],
    internalAddressList: [
    'yaLhoAZ4iex2zKmfvS9rvEmxXmRiPrjHdD',
    '...'
    ],
    otherAccountAddressList: []
};
```