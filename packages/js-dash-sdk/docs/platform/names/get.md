**Usage**: `client.platform.names.get(name)`    
**Description**: This method will allow you to fetch back a DPNS records from its humanized name. 

Parameters: 

| parameters                | type      | required       | Description                                                                   |  
|---------------------------|-----------|----------------| ----------------------------------------------------------------------------- |
| **name**                  | String    | yes            | An alphanumeric (2-63) value used for human-identification (can contains `-`) |

**Example**: `await client.platform.names.get('alice')`

Returns : Identity (or `null` if do not exist).
