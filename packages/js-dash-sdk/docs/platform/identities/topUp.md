**Usage**: `client.platform.identities.topUp(identity, amount)`    
**Description**: This method will topup the provided identity's balance. 

_The identity balance might slightly vary from the topped up amount due because of the transaction fee estimation._

Parameters: 

| parameters        | type    | required         | Description                                                                             |  
|-------------------|---------|------------------| ----------------------------------------------------------------------------------------|
| **identity**      | Identity| yes              | A valid [registered identity](../identities/register.md)                         |
| **amount**        | number  | yes              | A duffs (satoshis) value corresponding to the amount you want to top up to the identity.|

**Example**: 
```js 
const identityId = '';// Your identity identifier
const identity = await client.platform.identities.get(identityId);
await platform.identities.topUp(identity, 10000);

console.log(`New identity balance: ${identity.balance}`)
```

Returns : Boolean.
