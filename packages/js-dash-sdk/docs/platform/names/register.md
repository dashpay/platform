**Usage**: `client.platform.names.register(name, records, identity)`    
**Description**: This method will create a DPNS record matching your identity to the user or appname defined.

Parameters: 

| parameters                       | type      | required       | Description                                                                                                                                                                                 |  
|----------------------------------|-----------|----------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **name**                         | String    | yes            | An alphanumeric (1-63 character) value used for human-identification (can contain `-` but not as the first or last character). If a name with no parent domain is entered, '.dash' is used. |
| **records**                      | Object    | yes            | records object having only one of the following items                                                                                                                                       |
| **records.identity** | String    | yes             | Identity ID for this name record                                                                                                                                                     |
| **identity**                     | Identity  | yes            | A valid [registered identity](../identities/register.md)                                                                                                                           |


**Example**: `await client.platform.names.register('alice', { identity: identity.getId() }, identity)`

Returns: the created domain document
