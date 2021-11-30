**Usage**: `client.platform.contracts.update(contract, identity)`
**Description**: This method will sign and broadcast an updated valid contract.

Parameters:

| parameters                | type      | required       | Description                                                                   |
|---------------------------|-----------|----------------| ------------------------------------------------------------------------------|
| **contract**              | Contract  | yes            | A valid [created contract](/platform/contracts/create.md)                     |
| **identity**              | Identity  | yes            | A valid [registered `application` identity](/platform/identities/register.md) |

**Example**:
```js
const identityId = '';// Your identity identifier.
const dataContractId = ''; // Your existing contract id

// Retrieve existing data
const identity = await client.platform.identities.get(identityId);
const contract = await client.platform.contracts.get(dataContractId);

const contractDocuments = contract.getDocuments();

// Make necessary changes to contract document definitions
...

// and broadcast an update
await platform.contracts.update(contract, identity);
```

Returns : DataContractUpdateTransition.
