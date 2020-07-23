**Usage**: `client.platform.names.resolve(name.domain)`    
**Description**: This method will allow you to resolve a DPNS record from its humanized name. 

Parameters: 

| parameters                | type      | required       | Description                                                                   |  
|---------------------------|-----------|----------------| ----------------------------------------------------------------------------- |
| **name**                  | String    | yes            | An alphanumeric (2-63) value used for human-identification (can contains `-`) |

**Example**: `await client.platform.names.resolve('alice.dash')`

Returns : Document (or `null` if do not exist).
