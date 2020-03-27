**Usage**: `client.platform.identities.register('user')`    
**Description**: This method will create an `application` or `user` new identity for you. 

Parameters: 

| parameters        | type    | required            | Description                                                       |  
|-------------------|---------|------------------	| -----------------------------------------------------------------	|
| **identityType**  | string  | no (default: 'USER')| Allow to register a user (`USER`) or an application (`APPLICATION`) |

**Example**: `await client.platform.identities.register('APPLICATION')`
