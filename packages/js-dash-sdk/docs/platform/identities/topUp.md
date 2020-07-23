**Usage**: `client.platform.identities.topUp(identity, amount)`    
**Description**: This method will topup the provided identity's balance. 

Parameters: 

| parameters        | type    | required         | Description                                                                             |  
|-------------------|---------|------------------| ----------------------------------------------------------------------------------------|
| **identity**      | Identity| yes              | A valid [registered identity](/platform/identities/register.md)                         |
| **amount**        | number  | yes              | A duffs (satoshis) value corresponding to the amount you want to top up to the identity.|

**Example**: 
```js 
const identityId = '';// Your identity identifier
const identity = await client.platform.identities.get(identityId);
await platform.identities.topUp(identity, 10000);

console.log(`New identity balance: ${identity.balance}`)
```

Returns : Boolean.
