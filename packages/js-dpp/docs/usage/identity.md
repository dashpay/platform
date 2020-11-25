## dpp.identity.create(lockedOutPoint, publicKeys = [])

**Description**: Instantiate a new Identity.   

**Parameters**:

| parameters                   | type                   | required  | Description                                      |  
|------------------------------|------------------------|-----------| -------------------------------------------------|
| **assetLockTransaction**     | Transaction            | yes       |                                                  |
| **outputIndex**              | number                 | yes       |                                                  |
| **assetLockProof**           | InstantAssetLockProof  | yes       |                                                  |
| **publicKeys**               | PublicKey[]            | yes       |                                                  |

Returns : {[Identity](/primitives/Identity)}

## dpp.identity.createFromObject(rawIdentity, options)

**Description**: Instantiate a new Identity from plain object representation.   

**Parameters**:

| parameters                   | type            | required | Description                                             |  
|------------------------------|-----------------|----------| --------------------------------------------------------|
| **rawIdentity**              | RawIdentity     | yes      |                                                         |
| **options**                  | Object          | no       |                                                         |
| **options.skipValidation**   | boolean[=false] | no       |                                                         |

Returns : {[Identity](/primitives/Identity)}

## dpp.identity.createFromBuffer(buffer, options)

**Description**: Instantiate a new Identity from buffer.

**Parameters**:

| parameters                   | type            | required | Description                                             |  
|------------------------------|-----------------|----------| --------------------------------------------------------|
| **buffer**                   | Buffer          | yes      |                                                         |
| **options**                  | Object          | no       |                                                         |
| **options.skipValidation**   | boolean[=false] | no       |                                                         |

Returns : {[Identity](/primitives/Identity)}

## dpp.identity.validate(identity)

**Description**: Validate Identity

**Parameters**:

| parameters                   | type                         | required | Description                                             |  
|------------------------------|------------------------------|----------| --------------------------------------------------------|
| **identity**                 | Identity/RawIdentity         | yes      |                                                         |

Returns : {ValidationResult}

## dpp.identity.createInstantAssetLockProof(instantLock)

**Description**: Create a instant asset lock proof Identity.   

**Parameters**:

| parameters                   | type                   | required  | Description                                      |  
|------------------------------|------------------------|-----------| -------------------------------------------------|
| **instantLock**              | InstantLock            | yes       |                                                  |

Returns : {InstantAssetLookProof}

## dpp.identity.createIdentityCreateTransition(identity)

**Description**: Create Identity Create Transition

**Parameters**:

| parameters                   | type            | required | Description                                             |  
|------------------------------|-----------------|----------| --------------------------------------------------------|
| **identity**                 | Identity        | yes      |                                                         |

Returns : {IdentityCreateTransition}

## dpp.identity.createIdentityTopUpTransition(identityId, lockedOutPoint)

**Description**: Create Identity Create Transition

**Parameters**:

| parameters                   | type                     | required | Description                                             |  
|------------------------------|--------------------------|----------| --------------------------------------------------------|
| **identityId**               | Identifier/Buffer/String | yes      | identity id to top up                                   |
| **lockedOutPoint**           | Buffer                   | yes      | outpoint of the top up output of the L1 transaction     |

Returns : {IdentityTopUpTransition}
