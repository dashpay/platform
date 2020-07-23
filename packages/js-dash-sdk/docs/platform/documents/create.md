**Usage**: `client.platform.documents.create(typeLocator, identity, documentOpts)`    
**Description**: This method will return a Document object initialized with the parameters defined and apply to the used identity. 

Parameters: 

| parameters        | type    | required            | Description                                                       |  
|-------------------|---------|------------------	| -----------------------------------------------------------------	|
| **dotLocator**    | string  | yes                 | Field of a specific application, under the form `appName.fieldName` |
| **identity**      | Identity| yes                 | A valid [registered identity](/platform/identities/register.md) |
| **docOpts**       | Object  | yes                 | A valid data that match the data contract structure |

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
```
**Note**: When your document is created, it will only exist locally, use the [broadcast](/platform/documents/broadcast.md) method to register it.  

Returns: Document
