**Usage**: `client.platform.document.broadcast(documents, identity)`    
**Description**: This method will broadcast the document on the Application Chain

Parameters: 

| parameters                 | type       | required | Description                                                                  |  
|----------------------------|------------|----------|------------------------------------------------------------------------------|
| **documents**              | Object     | yes      |                                                                              |
| **documents.create**       | Document[] | no       | array of valid [created document](../documents/create.md) to create |
| **documents.replace**      | Document[] | no       | array of valid [created document](../documents/create.md) to replace         |
| **documents.delete**       | Document[] | no       | array of valid [created document](../documents/create.md) to delete          |
| **identity**               | Identity   | yes      | A valid [registered identity](../identities/register.md)                     |


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
