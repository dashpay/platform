**Usage**: `new DataTrigger(dataContractId, documentType, transitionAction, trigger, topLevelIdentity)`  
**Description**: Instantiate a DataTrigger.

**Parameters**:

| parameters                            | type                      | required           | Description               |  
|---------------------------------------|---------------------------|--------------------| --------------------------|
| **dataContractId**                    | Buffer/Identifier         | yes                |                           |
| **documentType**                      | string                    | yes                |                           |
| **transitionAction**                  | number                    | yes                |                           |
| **trigger**                           |string/DocumentTransition[]| yes                |                           |
| **topLevelIdentity**                  | Buffer/Identifier         | yes                |                           |

**Returns**: A new valid instance of DataTrigger

## .isMatchingTriggerForData(dataContractId, documentType, transitionAction)

**Description**: Check this trigger is matching for specified data

**Parameters**: 

| parameters                            | type                      | required           | Description               |  
|---------------------------------------|---------------------------|--------------------| --------------------------|
| **dataContractId**                    | string                    | yes                |                           |
| **documentType**                      | string                    | yes                |                           |
| **transitionAction**                  | number                    | yes                |                           |

**Returns**: {boolean}

## .execute(documentTransition, context)

**Description**: Execute data trigger

**Parameters**: 

| parameters                            | type                        | required           | Description               |  
|---------------------------------------|-----------------------------|--------------------| --------------------------|
| **documentTransition**                | DocumentTransition[]        | yes                |                           |
| **context**                           | DataTriggerExecutionContext | yes                |                           |

**Returns**: {Promise<DataTriggerExecutionResult>}}
