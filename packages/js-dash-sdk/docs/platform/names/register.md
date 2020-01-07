**Usage**: `sdk.platform.names.register(name, identity)`    
**Description**: This method will create a DPNS record matching your identity to the user or appname defined.

Parameters: 

| parameters                | type      | required       | Description                                                                   |  
|---------------------------|-----------|----------------| ----------------------------------------------------------------------------- |
| **name**                  | String    | yes            | An alphanumeric (2-63) value used for human-identification (can contains `-`) |
| **identity**              | Identity  | yes            | A valid [registered identity](/platform/identities/register.md)               |


**Example**: `await sdk.platform.identities.register('alice', identity)`
