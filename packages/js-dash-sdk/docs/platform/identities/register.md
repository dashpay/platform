**Usage**: `client.platform.identities.register()`    
**Description**: This method will register a new identity for you. 

Parameters: 

| parameters        | type    | required         | Description                                                        |  
|-------------------|---------|------------------| -------------------------------------------------------------------|
| fundingAmount     | number  | no               | Defaults: 10000. Allow to set a funding amount in duffs (satoshis).|

**Example**: `await client.platform.identities.register()`

**Note**: The created identity will be associated to the active account. You might want to know more about how to [change your active account](/examples/use-different-account).  

Returns : Identity.
