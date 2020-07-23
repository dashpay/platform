**Usage**: `client.platform.document.broadcast(documents, identity)`    
**Description**: This method will broadcast the document on the Application Chain

Parameters: 

| parameters                 | type       | required | Description                                                                 |  
|----------------------------|------------|----------| ----------------------------------------------------------------------------|
| **documents**              | Object     | yes      |                                                                             |
| **documents.create**       | Document[] | no       | array of valid [created document](/platform/documents/create.md) to create  |
| **documents.replace**      | Document[] | no       | array of valid [created document](/platform/documents/create.md) to replace |
| **documents.delete**       | Document[] | no       | array of valid [created document](/platform/documents/create.md) to delete  |
| **identity**               | Identity   | yes      | A valid [registered identity](/platform/identities/register.md)             |


**Example**:
```js
const identityId = '';// Your identity identifier
const identity = await client.platform.identities.get(identityId);

const helloWorldDocument = await platform.documents.create(
      // Assume a contract helloWorldContract is registered with a field note
      'helloWorldContract.note',
      identity,
     { message: 'Hello World'},
  );

await platform.documents.broadcast({create: [helloWorldDocument]}, identity);
```
Returns : documents.
