**Usage**: `client.platform.contracts.get(contractId)`    
**Description**: This method will allow you to fetch back a contract from it's id. 

Parameters: 

| parameters        | type    | required            | Description                                                       |  
|-------------------|---------|------------------	| -----------------------------------------------------------------	|
| **identifier**    | string  | yes                 | Will fetch back the contract matching the identifier |

**Example**: `await client.platform.contracts.get('77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3')`

Returns : Contract (or `null`).
