## new DashPlatformProtocol(options)

**Description**: Instantiate DashPlatformProtocol.

**Parameters**:

| parameters                                                | type               | required | Description                                            |  
|-----------------------------------------------------------|--------------------|----------| -------------------------------------------------------|
| **options**                                               | Object             | no       |                                                        |
| **options.stateRepository**                               | StateRepository    | no       |                                                        |
| **options.jsonSchemaValidator**                           | JsonSchemaValidator| no       |                                                        |

**Returns**: {DashPlatformProtocol}

**Notes**: DPP will provide multiples facades: 
- [dpp.dataContract](/usage/dataContract)
- [dpp.document](/usage/document)
- [dpp.identity](/usage/identity)
- [dpp.stateTransition](/usage/stateTransition)

## .getJsonSchemaValidator()

**Description**: Return JSON Schema Validator  

**Parameters**: None

**Returns**: {JsonSchemaValidator}

## .getStateRepository()

**Description**: Return State Repository  

**Parameters**: None

**Returns**: {StateRepository}

