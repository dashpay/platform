**Usage**: `client.platform.names.register(name, identity)`    
**Description**: This method will create a DPNS record matching your identity to the user or appname defined.

Parameters: 

| parameters                       | type      | required       | Description                                                                   |  
|----------------------------------|-----------|----------------| ----------------------------------------------------------------------------- |
| **name**                         | String    | yes            | An alphanumeric (1-63 character) value used for human-identification (can contain `-` but not as the first or last character). If a name with no parent domain is entered, '.dash' is used. |
| **records**                      | Object    | yes            | records object having only one of the following items                         |
| **records.dashUniqueIdentityId** | String    | no             | Unique Identity ID for this name record                                       |
| **records.dashAliasIdentityId**  | String    | no             | Used to signify that this name is the alias for another id                    |
| **identity**                     | Identity  | yes            | A valid [registered identity](/platform/identities/register.md)               |


**Example**: `await client.platform.identities.register('alice', { dashUniqueIdentityId: identity.getId() }, identity)`

Return: the created domain document
