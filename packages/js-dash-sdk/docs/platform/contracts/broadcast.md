**Usage**: `client.platform.contracts.broadcast(contract, identity)`    
**Description**: This method will sign and broadcast any valid contract. 

Parameters: 

| parameters                | type      | required       | Description                                                                   |  
|---------------------------|-----------|----------------| ------------------------------------------------------------------------------|
| **contract**              | Contract  | yes            | A valid [created contract](/platform/contracts/create.md)                     |
| **identity**              | Identity  | yes            | A valid [registered `application` identity](/platform/identities/register.md) |

**Example**:
```js
const identityId = '';// Your identity identifier.
const identity = await client.platform.identities.get(identityId);
// See the contract.create documentation for more on how to create a dataContract
const contract = await client.platform.contracts.create(contractDefinitions, identity);
await platform.contracts.broadcast(contract, identity);
```

Returns : dataContract.
