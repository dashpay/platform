**Usage**: `client.platform.document.broadcast(documents, identity)`    
**Description**: This method will broadcast the document on the Application Chain

Parameters: 

| parameters                 | type       | required | Description                                                                 |  
|----------------------------|------------|----------| ----------------------------------------------------------------------------|
| **documents**              | Object     | yes      |                                                                             |
| **documents.create**       | Document[] | yes      | array of valid [created document](/platform/documents/create.md) to create  |
| **documents.replace**      | Document[] | yes      | array of valid [created document](/platform/documents/create.md) to replace |
| **documents.delete**       | Document[] | yes      | array of valid [created document](/platform/documents/create.md) to delete  |
| **identity**               | Identity   | yes      | A valid [registered identity](/platform/identities/register.md)             |

Returns : documents.
