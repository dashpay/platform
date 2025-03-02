**Usage**: `client.platform.documents.broadcast(documents, identity, options)`    
**Description**: This method will broadcast a document transfer

Parameters: 

| parameters        | type    | required            | Description                                                           |  
|-------------------|---------|------------------	|-----------------------------------------------------------------------|
| **documents.transfer**    | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to transfer |
| **identity**      | Identity | yes                 | A valid [registered identity](../identities/register.md)              |
| **options**               | DocumentTransitionParams           | no       | An object with `receiver` field                                       |

**Example**: 
```js
const identityId = '';// Your identity identifier
const receiverId = ''; // Receiver identity identifier
const documentId = '' // Your document id

const identity = await client.platform.identities.get(identityId);
const receiverIdentity = await client.platform.identities.get(receiverId);

const [document] = await dash.platform.documents.get(
  'helloWorldContract.note',
  { where: [['$id', '==', documentId]] },
);

await dash.platform.documents.broadcast({ transfer: [document], }, identity, { receiver: receiverIdentity.getId() });
```

**Note**: Transfer transition changes the ownership of the given document to the receiver identity  

Returns: DocumentsBatchTransition
