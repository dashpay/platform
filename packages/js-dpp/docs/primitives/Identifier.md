**Usage**: `new Identifier(buffer)`  
**Description**: Instantiate a new Identifier.
Implements Buffer methods with base58 as default encoding. 

**Parameters**:

| parameters                            | type                  | required           | Description               |  
|---------------------------------------|-----------------------|--------------------| --------------------------|
| **buffer**                            | Buffer                | yes                |                           |

**Returns**: A new valid instance of Identifier

## .encodeCBOR(encoder)

**Description**: Encode to CBOR

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **encoder**        | Encoder                | yes                |                                  |

**Returns**: {boolean}

## .toJSON()

**Description**: Return Identity as JSON object

**Parameters**: None.  

**Returns**: {RawIdentity}

## .toBuffer()

**Description**: Convert to Buffer

**Parameters**: None.  

**Returns**: {Buffer}
