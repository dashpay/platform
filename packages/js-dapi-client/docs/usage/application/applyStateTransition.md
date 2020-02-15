**Usage**: `async client.applyStateTransition(stateTransition)`
**Description**: Send State Transition to machine

Parameters:

| parameters             | type                                                 | required       | Description                                                                                             |
|------------------------|------------------------------------------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **stateTransition**    | DataContractStateTransition/DocumentsStateTransition | yes            | A valid state transition |

Returns : Promise<!ApplyStateTransitionResponse>

