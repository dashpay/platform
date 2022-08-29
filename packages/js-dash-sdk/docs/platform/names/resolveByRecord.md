**Usage**: `client.platform.names.resolveByRecord(record, value)`    
**Description**: This method will allow you to resolve a DPNS record by identity ID. 

Parameters: 

| parameters | type      | required       | Description                                                          |  
|------------|-----------|----------------|----------------------------------------------------------------------|
| **record** | String    | yes            | Type of the record (`dashUniqueIdentityId` or `dashAliasIdentityId`) |
| **value**  | String    | yes            | Identifier value for the record                                      |

**Example**: 

This example will describe how to resolve names by the dash unique identity id.  
```js
const identityId = '3ge4yjGinQDhxh2aVpyLTQaoka45BkijkoybfAkDepoN';
const document = await client.platform.names.resolveByRecord('dashUniqueIdentityId', identityId);
```

Returns: array of Document.
