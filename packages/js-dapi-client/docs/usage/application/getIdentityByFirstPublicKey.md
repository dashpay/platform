**Usage**: `async client.platform.getIdentityByFirstPublicKey(publicKeyHash)`
**Description**: Fetch the identity using the public key hash of the identity's first key

Parameters:

| parameters             | type               | required       | Description                                                                                      |
|------------------------|--------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **publicKeyHash**      | String             | yes            | A valid public key hash |

Returns : Promise<!Buffer|null>
