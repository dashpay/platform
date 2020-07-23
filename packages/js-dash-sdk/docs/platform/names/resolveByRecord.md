**Usage**: `client.platform.names.resolve(name.domain)`    
**Description**: This method will allow you to resolve a DPNS record from its identity ID. 

Parameters: 

| parameters                | type      | required       | Description                                                                   |  
|---------------------------|-----------|----------------| ----------------------------------------------------------------------------- |
| **name**                  | String    | yes            | An alphanumeric (2-63) value used for human-identification (can contains `-`) |

**Example**: 

This example will describe how to resolve names by the identity id, but other records field will works too.  
```js
const identityId = '3ge4yjGinQDhxh2aVpyLTQaoka45BkijkoybfAkDepoN';
const document = await client.platform.names.resolveByRecord('dashIdentity',identityId);
```

Returns : Document (or `null` if do not exist).
