## dpp.document.create(dataContract, ownerId, type, data = {})

**Description**: Instantiate a new Document for a specific contract, owner, type.
This method will populate it with specified data and validate upon creation.

**Parameters**:

| parameters                   | type            | required  | Description                                            |  
|------------------------------|-----------------|-----------| -------------------------------------------------------|
| **dataContract**             | DataContract    | yes       |                                                        |
| **ownerId**                  | Buffer          | yes       |                                                        |
| **type**                     | string          | yes       |                                                        |
| **data**                     | Object[={}]     | no        |                                                        |

Returns : {[Document](/primitives/Document)}

## dpp.document.createFromObject(rawDocument, options)

**Description**: Instantiate a new Document from plain object representation.   
By default, the provided Document will be validated. 

**Parameters**:

| parameters                   | type            | required | Description                                             |  
|------------------------------|-----------------|----------| --------------------------------------------------------|
| **rawDocument**              | RawDocument     | yes      |                                                         |
| **options**                  | Object          | no       |                                                         |
| **options.skipValidation**   | boolean[=false] | no       |                                                         |
| **options.action**           | boolean         | no       |                                                         |

Returns : {Promise<[Document](/primitives/Document)>}

## dpp.document.createFromBuffer(buffer, options)

**Description**: Instantiate a new Document from buffer.   

**Parameters**:

| parameters                   | type            | required | Description                                             |  
|------------------------------|-----------------|----------| --------------------------------------------------------|
| **buffer**                   | Buffer          | yes      |                                                         |
| **options**                  | Object          | no       |                                                         |
| **options.skipValidation**   | boolean[=false] | no       |                                                         |
| **options.action**           | boolean         | no       |                                                         |

Returns : {Promise<[Document](/primitives/Document)>}

## dpp.document.createStateTransition(documents)

**Description**: Create Documents State Transition

**Parameters**:

| parameters                   | type            | required | Description                                             |  
|------------------------------|-----------------|----------| --------------------------------------------------------|
| **documents**                | Object          | yes      |                                                         |
| **documents.create**         | Document[]      | no       |                                                         |
| **documents.replace**        | Document[]      | no       |                                                         |
| **documents.delete**         | Document[]      | no       |                                                         |

Returns : {DocumentsBatchTransition}

## dpp.document.validate(document)

**Description**: Validate document

**Parameters**:

| parameters      | type                 | required | Description                                             |  
|-----------------|----------------------|----------| --------------------------------------------------------|
| **document**    | Document/RawDocument | yes      |                                                         |

Returns : {Promise<ValidationResult>}
