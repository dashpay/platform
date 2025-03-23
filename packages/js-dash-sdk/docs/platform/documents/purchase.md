**Usage**: `client.platform.documents.broadcast(documents, identity, options)`    
**Description**: This method will broadcast a purchase state transition that buys the given document from other Identity.

Parameters: 

| parameters             | type    | required            | Description                                                       |  
|------------------------|---------|------------------	|-------------------------------------------------------------------|
| **documents.purchase** | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to buy  |
| **identity**           | Identity | yes                 | A valid [registered identity](../identities/register.md)          |
| **options**            | DocumentTransitionParams           | no       | An object with field `price` (BigInt) and `receiver` (Identifier) |

**Example**: 
```js
const identityId = '';// Your identity identifier
const receiverId = ''; // Receiver identity identifier
const documentId = '' // Your document id
const price = BigInt(1000000)

const identity = await client.platform.identities.get(identityId);
const receiverIdentity = await client.platform.identities.get(receiverId);

const identity = await client.platform.identities.get(identityId);

const [document] = await dash.platform.documents.get(
  'helloWorldContract.note',
  { where: [['$id', '==', documentId]] },
);

await dash.platform.documents.broadcast({ purchase: [document], }, identity, { price, receiver: receiverIdentity.getId() });
```
**Note**: This method will change the ownership of the document to your identity, and seller identity will be credited with the amount specified in the updatePrice deducted from your balance.

Returns: DocumentsBatchTransition
