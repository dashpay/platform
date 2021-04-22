**Usage**: `identities.getIdentityHDKeyByIndex(identityIndex, keyIndex)`      
**Description**: This method returns the identity HDKey of `identityIndex` for the specified `keyIndex`  

Parameters: 

| parameters          | type      | required       | Description                                                                     |  
|---------------------|-----------|----------------| --------------------------------------------------------------------------------|
| **identityIndex**   | number    | yes            | To derive the key for a specific identityIndex (default: 0)                     |
| **keyIndex**        | number    | yes            | To derive the key for a specific keyIndex (default: 0)                          |

Returns : HDKeys (private, public).
