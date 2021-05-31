**Usage**: `new DataContract(rawDataContract)`  
**Description**: Instantiate a DataContract.

**Parameters**:

| parameters                            | type             | required           | Description               |  
|---------------------------------------|------------------|--------------------| --------------------------|
| **rawDataContract**                   | RawDataContract  | yes                |                           |
| **rawDataContract.$id**               | Buffer           | yes                |                           |
| **rawDataContract.$schema**           | string           | yes                |                           |
| **rawDataContract.protocolVersion**   | number           | yes                |                           |
| **rawDataContract.ownerId**           | Buffer           | yes                |                           |
| **rawDataContract.documents**         | Object<str, obj> | yes                |                           |
| **rawDataContract.$defs**       | Object<str, obj> | no                 |                           |

**Returns**: A new valid instance of DataContract

## .getProtocolVersion()

**Description**: Get Data Contract protocol version

**Parameters**: None.  

**Returns**: {number}

## .getId()

**Description**: Get Data Contract id

**Parameters**: None.  

**Returns**: {Identifier}

## .getOwnerId()

**Description**: Get Data Contract owner id

**Parameters**: None.  

**Returns**: {Identifier}

## .getJsonSchemaId()

**Description**: Get Data Contract JSON Schema ID

**Parameters**: None.  

**Returns**: {string}

## .getJsonMetaSchema()

**Description**: Get Data Contract JSON Meta Schema

**Parameters**: None.  

**Returns**: {string}

## .setJsonMetaSchema(schema)

**Description**: Allow to set JSON Meta Schema to this DataContract (overwrite previous value).

**Parameters**:  

| parameters            | type            | required           | Description                      |  
|-----------------------|-----------------|--------------------| -------------------------------- |
| **schema**            | string          | yes                |                                  |

**Returns**: {DataContract}

## .setDocuments(documents)

**Description**: Set documents for this DataContract (overwrite previous value).

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **documents**      | Object<string, Object> | yes                |                                  |

**Returns**: {DataContract}

## .getDocuments()

**Description**: Get Data Contract documents

**Parameters**: None.  

**Returns**: {Object<string, Object>} - documents

## .isDocumentDefined(type)

**Description**: Returns true if document type has been defined

**Parameters**:  

| parameters         | type    | required           | Description                      |  
|--------------------|---------|--------------------| -------------------------------- |
| **type**           | string  | yes                |                                  |

**Returns**: {Boolean} - whether document type has been defined

## .setDocumentSchema(type, schema)

**Description**: Setter for document schema.

**Parameters**:  

| parameters         | type    | required           | Description                      |  
|--------------------|---------|--------------------| -------------------------------- |
| **type**           | string  | yes                |                                  |
| **schema**         | object  | yes                |                                  |

**Returns**: {DataContract}

## .getDocumentSchema(type)

**Description**: Get Data Contract Document Schema for the provided type

**Parameters**:  

| parameters         | type    | required           | Description                      |  
|--------------------|---------|--------------------| -------------------------------- |
| **type**           | string  | yes                |                                  |

**Returns**: {Object} - document

## .getDocumentSchemaRef(type)

**Description**: Get Data Contract Document schema reference

**Parameters**:  

| parameters         | type    | required           | Description                      |  
|--------------------|---------|--------------------| -------------------------------- |
| **type**           | string  | yes                |                                  |

**Returns**: {{$ref: string}} - reference

## .setDefinitions($defs)

**Description**: Setter for $defs.

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **$defs**    | Object<string, Object> | yes                |                                  |

**Returns**: {DataContract}

## .getDefinitions()

**Description**: Get Data Contract $defs

**Parameters**: None.  

**Returns**: {Object<string, Object>} - $defs

## .getBinaryProperties(type)

**Description**: Get properties with `contentEncoding` constraint

**Parameters**:  

| parameters         | type    | required           | Description                      |  
|--------------------|---------|--------------------| -------------------------------- |
| **type**           | string  | yes                |                                  |

**Returns**: {Object}

## .toObject(options)

**Description**: Return Data Contract as plain object

**Parameters**:  

| parameters                | type    | required | Description                      |  
|---------------------------|---------|----------| -------------------------------- |
| **options**               | Object  | no       |                                  |
| **options.skipIdentifiersConversion** | Boolean | no       |                                  |

**Returns**: {RawDataContract}

## .toJSON()

**Description**: Return Data Contract as JSON object

**Parameters**: None.  

**Returns**: {JsonDataContract}

## .toBuffer()

**Description**: Return Data Contract as a Buffer

**Parameters**: None.  

**Returns**: {Buffer}

## .hash()

**Description**: Returns Data Contract hash

**Parameters**: None.  

**Returns**: {Buffer}

## .setEntropy(entropy)

**Description**: Set Data Contract entropy

**Parameters**:  

| parameters         | type                   | required           | Description                      |  
|--------------------|------------------------|--------------------| -------------------------------- |
| **entropy**        | Buffer                 | yes                |                                  |

**Returns**: {DataContract}

## .getEntropy()

**Description**: Get Data Contract entropy

**Parameters**: None.  

**Returns**: {Buffer}
