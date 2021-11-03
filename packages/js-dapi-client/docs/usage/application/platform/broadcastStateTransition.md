**Usage**: `async client.platform.broadcastStateTransition(stateTransition, options)`
**Description**: Send State Transition to machine

Parameters:

| parameters             | type              | required       | Description                                                                                      |
|------------------------|-------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **stateTransition**    | Buffer            | yes            | A valid bufferized state transition |
| **options**            | DAPIClientOptions | no             | A valid state transition |

Returns : Promise<!BroadcastStateTransitionResponse>
