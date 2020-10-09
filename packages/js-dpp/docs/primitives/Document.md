**Usage**: `new Document(rawDocument)`  
**Description**: Instantiate a Document.

**Parameters**:

| parameters                            | type            | required           | Description               |  
|---------------------------------------|-----------------|--------------------| --------------------------|
| **rawDocument**                       | RawDocument     | yes                |                           |
| **rawDocument.$id**                   | Buffer          | yes                |                           |
| **rawDocument.$dataContractId**       | string          | yes                |                           |
| **rawDocument.$protocolVersion**      | number          | yes                |                           |
| **rawDocument.$type**                 | string          | yes                |                           |
| **rawDocument.$ownerId**              | Buffer          | yes                |                           |
| **rawDocument.$revision**             | number          | yes                |                           |
| **rawDocument.$createdAt**            | number          | no                 |                           |
| **rawDocument.$updatedAt**            | number          | no                 |                           |
| **dataContract**                      | DataContract    | yes                |                           |

**Returns**: A new valid instance of Document

## Document.fromJSON(jsonDocument, dataContract)

**Description**: Instantiate a Document.

**Parameters**: 

| parameters                            | type            | required           | Description               |  
|---------------------------------------|-----------------|--------------------| --------------------------|
| **jsonDocument**                      | JsonDocument    | yes                |                           |
| **dataContract**                      | DataContract    | yes                |                           |  

**Returns**: {Document} - A new valid instance of Document

## .getProtocolVersion()

**Description**: Get Document protocol version

**Parameters**: None.  

**Returns**: {number}

## .getId()

**Description**: Get Document id

**Parameters**: None.  

**Returns**: {EncodedBuffer}

## .getType()

**Description**: Get Document type

**Parameters**: None.  

**Returns**: {string}

## .getDataContractId()

**Description**: Get Document Contract Id

**Parameters**: None.  

**Returns**: {EncodedBuffer}

## .getDataContract()

**Description**: Get Document Data Contract

**Parameters**: None.  

**Returns**: {DataContract}

## .getOwnerId()

**Description**: Get Document owner id

**Parameters**: None.  

**Returns**: {EncodedBuffer}

## .setRevision(revision)

**Description**: Set Document revision

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **revision**       | number                 | yes                |                                  |

**Returns**: {Document}

## .getRevision()

**Description**: Get Document revision

**Parameters**: None.  

**Returns**: {number}

## .setEntropy(entropy)

**Description**: Set Document entropy

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **entropy**        | Buffer                 | yes                |                                  |

**Returns**: {Document}

## .getEntropy()

**Description**: Get Document entropy

**Parameters**: None.  

**Returns**: {EncodedBuffer}

## .setData(data)

**Description**: Set document data (overwrite any previous data set)

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **data**           | Object                 | yes                |                                  |

**Returns**: {Document}

## .getData()

**Description**: Get Document data

**Parameters**: None.  

**Returns**: {Object}

## .get(path)

**Description**: Retrieves the field specified by path

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **path**           | String                 | yes                |                                  |

**Returns**: {*}

## .set(path, value)

**Description**: Set the field specified by {path}

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **path**           | String                 | yes                |                                  |
| **value**          | *                      | yes                |                                  |

**Returns**: {Document}

## .setCreatedAt(date)

**Description**: Set document creation date

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **date**           | Date                   | yes                |                                  |

**Returns**: {Document}

## .getCreatedAt()

**Description**: Get document creation date

**Parameters**: None.  

**Returns**: {Date}

## .setUpdatedAt(date)

**Description**: Set document updated date

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **date**           | Date                   | yes                |                                  |

**Returns**: {Document}

## .getUpdatedAt()

**Description**: Get document updated date

**Parameters**: None.  

**Returns**: {Date}

## .toJSON()

**Description**: Return Document as JSON object

**Parameters**: None.  

**Returns**: {JsonDocument}

## .toObject(options)

**Description**: Return Document as plain object (without converting encoded fields)

**Parameters**:  

| parameters                | type                   | required           | Description                      |  
|---------------------------|------------------------|--------------------| -------------------------------- |
| **options**               | Object                 | no                 |                                  |
| **options.skipIdentifiersConversion** | boolean[=false]        | no                 |                                  |

**Returns**: {RawDocument}

## .toBuffer()

**Description**: Return serialized Document as buffer

**Parameters**: None.  

**Returns**: {Buffer}

## .hash()

**Description**: Returns hex string with Document hash

**Parameters**: None.  

**Returns**: {string}
