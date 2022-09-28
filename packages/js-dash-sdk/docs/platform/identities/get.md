**Usage**: `client.platform.identities.get(identityId)`    
**Description**: This method will allow you to fetch back an identity from its id. 

Parameters: 

| parameters        | type    | required            | Description                                                       |  
|-------------------|---------|------------------	| -----------------------------------------------------------------	|
| **identifier**    | string  | yes                 | Will fetch back the identity matching the identifier              |

**Example**: `await client.platform.identities.get('3GegupTgRfdN9JMS8R6QXF3B2VbZtiw63eyudh1oMJAk')`

Returns: Identity (or `null` if it does not exist).
