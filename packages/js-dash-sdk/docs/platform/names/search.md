**Usage**: `client.platform.names.search(labelPrefix, parentDomain)`    
**Description**: This method will allow you to search all records matching the label prefix on the specified parent domain. 

Parameters: 

| parameters                | type      | required       | Description                                                                   |  
|---------------------------|-----------|----------------| ----------------------------------------------------------------------------- |
| **labelPrefix**           | String    | yes            | label prefix to search for                                                    |
| **parentDomain**          | String    | yes            | parent domain name on which to perform the search                             |

**Example**: 

This example will describe how to search all names on the parent domain `dash` that starts with the label prefix `al`. 
It will resolves names documents such as `alice`, `alex` etc...   

```js
const labelPrefix = 'al';
const parentDomain = 'dash';
const document = await client.platform.names.search(labelPrefix, parentDomain);
```

Returns : Documents matching the label prefix on the parent domain.
