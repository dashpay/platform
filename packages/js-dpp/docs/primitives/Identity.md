**Usage**: `new Identity(rawIdentity)`  
**Description**: Instantiate a new Identity.

**Parameters**:

| parameters                            | type                  | required           | Description               |  
|---------------------------------------|-----------------------|--------------------| --------------------------|
| **rawIdentity**                       | RawIdentity           | yes                |                           |
| **rawIdentity.id**                    | Buffer                | yes                |                           |
| **rawIdentity.protocolVersion**       | number                | yes                |                           |
| **rawIdentity.publicKeys**            | RawIdentityPublicKey[]| yes                |                           |
| **rawIdentity.balance**               | number                | yes                |                           |
| **rawIdentity.revision**              | number                | yes                |                           |

**Returns**: A new valid instance of Identity

## .getProtocolVersion()

**Description**: Get Identity protocol version

**Parameters**: None.  

**Returns**: {number}

## .getId()

**Description**: Get Identity id

**Parameters**: None.  

**Returns**: {Identifier}

## .setPublicKeys(publicKeys)

**Description**: Set Identity public keys

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **publicKeys**     | RawIdentityPublicKey[] | yes                |                                  |

**Returns**: {Identity}

## .getPublicKeys()

**Description**: Get Identity public keys revision

**Parameters**: None.  

**Returns**: {IdentityPublicKey[]}

## .getPublicKeyById(keyId)

**Description**: Returns a public key for a given id

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **keyId**          | number                 | yes                |                                  |

**Returns**: {IdentityPublicKey}

## .getBalance()

**Description**: Returns balance

**Parameters**: None.  

**Returns**: {number}

## .setBalance(balance)

**Description**: Set Identity balance

**Parameters**:  

| parameters         | type      | required           | Description                      |  
|--------------------|-----------|--------------------| -------------------------------- |
| **balance**        | number    | yes                |                                  |

**Returns**: {Identity}

## .increaseBalance(amount)

**Description**: Increase Identity balance

**Parameters**:  

| parameters         | type   | required           | Description                      |  
|--------------------|--------|--------------------| -------------------------------- |
| **amount**         | number | yes                |                                  |

**Returns**: {Identity}

## .reduceBalance(amount)

**Description**: Reduce Identity balance

**Parameters**:  

| parameters         | type   | required           | Description                      |  
|--------------------|--------|--------------------| -------------------------------- |
| **amount**         | number | yes                |                                  |

**Returns**: {Identity}

## .setLockedOutPoint(lockedOutPoint)

**Description**: Set Identity locked out point

**Parameters**:  

| parameters         | type   | required           | Description                      |  
|--------------------|--------|--------------------| -------------------------------- |
| **lockedOutPoint** | Buffer | yes                |                                  |

**Returns**: {Identity}

## .getLockedOutPoint()

**Description**: Returns Identity locked out point 

**Parameters**: None.  

**Returns**: {Buffer}

## .setRevision(revision)

**Description**: Set Identity revision

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **revision**       | number                 | yes                |                                  |

**Returns**: {Identity}

## .getRevision()

**Description**: Get Identity revision

**Parameters**: None.  

**Returns**: {number}

## .toObject()

**Description**: Return Identity as plain object

**Parameters**: None.  

**Returns**: {Object}

## .toJSON()

**Description**: Return Identity as JSON object

**Parameters**: None.  

**Returns**: {RawIdentity}

## .toBuffer()

**Description**: Return Identity as Buffer

**Parameters**: None.  

**Returns**: {Buffer}

## .hash()

**Description**: Returns Identity hash

**Parameters**: None.  

**Returns**: {Buffer}
