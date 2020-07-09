**Usage**: `async client.platform.getIdentityIdByFirstPublicKey(publicKeyHash)`
**Description**: Fetch the identity ID using the public key hash of the identity's first key

Parameters:

| parameters             | type               | required       | Description                                                                                      |
|------------------------|--------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **publicKeyHash**      | String             | yes            | A valid public key hash |

Returns : Promise<!Buffer|null>
