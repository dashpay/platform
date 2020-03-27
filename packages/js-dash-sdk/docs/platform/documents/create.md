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
  // See https://github.com/dashevo/dpns-contract on how to get those value.
  const dpnsDocOpts =  {nameHash, label,normalizedLabel,normalizedParentDomainName,preorderSalt,records };
  const document = client.platform.documents.create('dpns.domain', identity, dpnsDocOpts);
```
