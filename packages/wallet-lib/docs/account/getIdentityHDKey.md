**Usage**: `account.getIdentityHDKey(index, identityType)`      
**Description**: This method return the identity HDKey of `identityType` for the specified `index`  

Parameters: 

| parameters          | type      | required       | Description                                                                     |  
|---------------------|-----------|----------------| --------------------------------------------------------------------------------|
| **index**           | number    | no             | To derive the key on a specific index (default: 0)                              |
| **identityType**    | string    | no             | Either 'USER' or 'APPLICATION', defaults: 'USER'                                |

Returns : HDKeys (private, public).
