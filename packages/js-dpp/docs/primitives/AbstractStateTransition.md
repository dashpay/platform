**Usage**: `new AbstractStateTransition(rawStateTransition)`  

**Description**: Instantiate a new AbstractStateTransition.

**Parameters**:

| parameters                            | type                  | required           | Description               |  
|---------------------------------------|-----------------------|--------------------| --------------------------|
| **rawStateTransition**                | RawStateTransition    | yes                |                           |
| **rawStateTransition.protocolVersion**| number                | yes                |                           |
| **rawStateTransition.type**           | number                | yes                |                           |
| **rawStateTransition.signature**      | string/null           | yes                |                           |

**Returns**: A new valid instance of AbstractStateTransition

## .getProtocolVersion()

**Description**: Get protocol version

**Parameters**: None.  

**Returns**: {number}

## .getSignature()

**Description**: Returns signature

**Parameters**: None.  

**Returns**: {EncodedBuffer|null}

## .setSignature(signature)

**Description**: Set signature

**Parameters**: 

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **signature**      | Buffer                 | no                 |                                  |

**Returns**: {AbstractStateTransition}

## .getId()

**Description**: Get State Transition id

**Parameters**: None.  

**Returns**: {Buffer}

## .signByPrivateKey(privateKey)

**Description**: Sign data with private key

**Parameters**: 

| parameters         | type                                | required | Description                               |  
|--------------------|-------------------------------------|----------| ----------------------------------------- |
| **signature**      | string/Buffer/Uint8Array/PrivateKey | no       |  privateKey string must be hex or base58  |

**Returns**: {AbstractStateTransition}

## .verifySignatureByPublicKey(privateKey)

**Description**: Verify signature with private key

**Parameters**: 

| parameters         | type                                | required | Description                               |  
|--------------------|-------------------------------------|----------| ----------------------------------------- |
| **signature**      | string/Buffer/Uint8Array/PrivateKey | no       |  privateKey string must be hex or base58  |

**Returns**: {boolean}

## .calculateFee()

**Description**: Calculate ST fee in credits

**Parameters**: None.

**Returns**: {number}

## .toObject(options)

**Description**: Return state transition as plain object

**Parameters**: 

| parameters               | type                   | required           | Description                      |  
|--------------------------|------------------------|--------------------| -------------------------------- |
| **options**              | Object                 | no                 |                                  |
| **options.skipSignature**| Boolean[=false]        | no                 |                                  |
| **options.skipIdentifiersConversion**| Boolean[=false]        | no                 |                                  |

**Returns**: {RawStateTransition}

## .toJSON()

**Description**: Return state transition as JSON object

**Parameters**: None.

**Returns**: {JsonStateTransition}

## .toBuffer(options)

**Description**: Return serialized State Transition as buffer

**Parameters**:  

| parameters               | type                   | required           | Description                      |  
|--------------------------|------------------------|--------------------| -------------------------------- |
| **options**              | Object                 | no                 |                                  |
| **options.skipSignature**| Boolean[=false]        | no                 |                                  |

**Returns**: {Buffer}

## .hash()

**Description**: Returns hex string hash

**Parameters**: None.  

**Returns**: {string}
