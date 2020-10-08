## dpp.stateTransition.createFromJSON(rawStateTransition, options)

**Description**: Create State Transition from JSON.

**Parameters**:

| parameters                   | type                                                        | required | Description                                             |  
|------------------------------|-------------------------------------------------------------|----------| --------------------------------------------------------|
| **rawStateTransition**       | RawDataContractCreateTransition/RawDocumentsBatchTransition | yes      |                                                         |
| **options**                  | Object                                                      | no       |                                                         |
| **options.skipValidation**   | boolean[=false]                                             | no       |                                                         |

Returns : {RawDataContractCreateTransition|DocumentsBatchTransition}

# dpp.stateTransition.createFromObject(rawStateTransition, options)

**Description**: Create State Transition from a plain object.

**Parameters**:

| parameters                   | type                                                        | required | Description                                             |  
|------------------------------|-------------------------------------------------------------|----------| --------------------------------------------------------|
| **rawStateTransition**       | RawDataContractCreateTransition/RawDocumentsBatchTransition | yes      |                                                         |
| **options**                  | Object                                                      | no       |                                                         |
| **options.skipValidation**   | boolean[=false]                                             | no       |                                                         |

Returns : {RawDataContractCreateTransition|DocumentsBatchTransition}

## dpp.stateTransition.createFromBuffer(buffer, options)

**Description**: Create State Transition from buffer.

**Parameters**:

| parameters                   | type            | required | Description                                             |  
|------------------------------|-----------------|----------| --------------------------------------------------------|
| **buffer**                   | Buffer          | yes      |                                                         |
| **options**                  | Object          | no       |                                                         |
| **options.skipValidation**   | boolean[=false] | no       |                                                         |

Returns : {RawDataContractCreateTransition|DocumentsBatchTransition}

## dpp.stateTransition.validate(stateTransition)

**Description**: Validate State Transition

**Parameters**:

| parameters                   | type                                      | required | Description                                             |  
|------------------------------|-------------------------------------------|----------| --------------------------------------------------------|
| **stateTransition**          | RawStateTransition/AbstractStateTransition| yes      |                                                         |

Returns : {ValidationResult}

## dpp.stateTransition.validateStructure(stateTransition)

**Description**: Validate State Transition Structure

**Parameters**:

| parameters                   | type                                       | required | Description                                             |  
|------------------------------|--------------------------------------------|----------| --------------------------------------------------------|
| **stateTransition**          | AbstractStateTransition/RawStateTransition | yes      |                                                         |

Returns : {ValidationResult}

## dpp.stateTransition.validateDate(stateTransition)

**Description**: Validate State Transition Date

**Parameters**:

| parameters                   | type                      | required | Description                                             |  
|------------------------------|---------------------------|----------| --------------------------------------------------------|
| **stateTransition**          | AbstractStateTransition   | yes      |                                                         |

Returns : {ValidationResult}

## dpp.stateTransition.validateFee(stateTransition)

**Description**: Validate State Transition Fee

**Parameters**:

| parameters                   | type                    | required | Description                                             |  
|------------------------------|-------------------------|----------| --------------------------------------------------------|
| **stateTransition**          | AbstractStateTransition | yes      |                                                         |

Returns : {ValidationResult}

## dpp.stateTransition.apply(stateTransition)

**Description**: Apply state transition to the state

**Parameters**:

| parameters                   | type                    | required | Description                                      |  
|------------------------------|-------------------------|----------| -------------------------------------------------|
| **stateTransition**          | AbstractStateTransition | yes      |                                                  |

Returns : {Promise<void>}
