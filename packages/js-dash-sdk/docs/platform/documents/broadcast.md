**Usage**: `client.platform.document.broadcast(documents, identity)`    
**Description**: This method will broadcast the document on the Application Chain

Parameters: 

| parameters                | type               | required | Description                                                                            |  
|---------------------------|--------------------|----------|----------------------------------------------------------------------------------------|
| **documents**             | Object             | yes      |                                                                                        |
| **documents.create**      | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to create                    |
| **documents.replace**     | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to replace                   |
| **documents.delete**      | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to delete                    |
| **documents.transfer**    | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to transfer                  |
| **documents.updatePrice** | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to set price                 |
| **documents.purchase**    | ExtendedDocument[] | no       | array of valid [created document](../documents/create.md) to purchase                  |
| **identity**              | Identity           | yes      | A valid [registered identity](../identities/register.md)                               |
| **options**               | DocumentTransitionParams           | no       | An object with two optional fields `price` and `receiver` that is used for NFT actions |


**Example**:
```js
const identityId = '';// Your identity identifier
const identity = await client.platform.identities.get(identityId);

const helloWorldDocument = await client.platform.documents.create(
    // Assuming a contract tutorialContract is registered with a field note
    'tutorialContract.note',
    identity,
    { message: 'Hello World'},
);

await client.platform.documents.broadcast({ create: [helloWorldDocument] }, identity);
```
Returns: documents.
