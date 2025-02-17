**Usage**: `client.platform.documents.broadcast(documents, identity, options)`    
**Description**: This method will broadcast an update price state transition that sets a price for the given document.

Parameters: 

| parameters                | type    | required            | Description                                                               |  
|---------------------------|---------|------------------	|---------------------------------------------------------------------------|
| **documents.updatePrice** | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to update price |
| **identity**              | Identity | yes                 | A valid [registered identity](../identities/register.md)                  |
| **options**               | DocumentTransitionParams           | no       | An object with field `price` (BigInt)                                     |

**Example**: 
```js
const identityId = '';// Your identity identifier
const documentId = '' // Your document id
const price = BigInt(1000000)

const identity = await client.platform.identities.get(identityId);

const [document] = await dash.platform.documents.get(
  'helloWorldContract.note',
  { where: [['$id', '==', documentId]] },
);

await dash.platform.documents.broadcast({ updatePrice: [document], }, identity, { price });
```
**Note**: This method sets the same price on all documents in the batch (only one is possible right now)

Returns: DocumentsBatchTransition
