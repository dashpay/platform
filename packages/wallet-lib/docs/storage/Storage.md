**Usage**: `new Storage(opts)`  
**Description**: This method create a new Storage instance which provide various helper with interacting with atomic elements (tx, address). It connects with the adapter to perform load/save operations.
   

Parameters: 

- opts : 

| parameters                         | type            | required       | Description                                                                                                                                                                    |  
|------------------------------------|-----------------|----------------| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **opts.network**                   | Network|String  | no (testnet)   | The network to use for the Storage instance                                                          |
| **opts.rehydrate**                 | Boolean         | no (true)      | If data should be autoloaded from the adapter                                                        |
| **opts.autosave**                  | Boolean         | no (true)      | If set at true, will autosave the storage to adapter at an autosaveIntervalTime                      |
| **opts.autosaveIntervalTime**      | Number          | no (10sec)     | If millisecond, the interval time at which the adapter should persist the data                       |

Returns : Storage instance.

