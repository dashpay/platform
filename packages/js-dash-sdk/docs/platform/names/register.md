**Usage**: `client.platform.names.register(name, identity)`    
**Description**: This method will create a DPNS record matching your identity to the user or appname defined.

Parameters: 

| parameters                | type      | required       | Description                                                                   |  
|---------------------------|-----------|----------------| ----------------------------------------------------------------------------- |
| **name**                  | String    | yes            | An alphanumeric (1-63 character) value used for human-identification (can contain `-` but not as the first or last character). If a name with no parent domain is entered, '.dash' is used. |
| **identity**              | Identity  | yes            | A valid [registered identity](/platform/identities/register.md)               |


**Example**: `await client.platform.identities.register('alice', identity)`
