**Usage**: `storage.configure(opts)`    
**Description**: After Storage creation, this method is called to ensure Adapter contains expected method.    
**Notes**: This is an internal advanced function called on the startup of a Storage.       

Parameters: 

| parameters             | type              | required         | Description                                                             |  
|------------------------|-------------------|------------------| ------------------------------------------------------------------------|
| **opts.rehydrate**     | Boolean           |  no              |  Set if the Storage will autoload from the adapter                      |
| **opts.autosave**      | Boolean           |  no              |  Set if the Storage will autosave to the adapter                        |
| **opts.adapter**       | Adapter           |  no              |  The adapter to test and use.                                           |


Returns: void.  
Emits: `CONFIGURED`

