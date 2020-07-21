**Usage**: `storage.updateTransaction(transaction)`      
**Description**: Internally, this is mostly called to update the information of a transaction in the store. Works mostly more as an replace than an update.   

Parameters: 

| parameters             | type              | required       | Description                                               |  
|------------------------|-------------------|----------------| ----------------------------------------------------------|
| **transaction**        | Transaction       | yes            | The Transaction to update (uses tx.hash as key)           |

Returns : Boolean.

Example: 
```js
const { Transaction } = require('@dashevo/dashcore-lib');
const transaction = new Transaction({
     hash: '9b4a34096f2270f70d8e0ba91094eb37535349f80874f8440e74c0567ef82680',
     version: 3,
     inputs: [
       {
         prevTxId: '9f398515b6fc898ebf4e7b49bbfc4359b8c89f508c6cd677e53946bd86064b28',
         outputIndex: 0,
         sequenceNumber: 4294967295,
         script: '47304402205bb4f7880fb0fc13218940ba341c30e817363e5590343d28639af921b2a5f1d40220010920ae4b00bbb657f8653cb44172b8cb13447bb5105ddaf32a2845ea0666b90121025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b',
         scriptString: '71 0x304402205bb4f7880fb0fc13218940ba341c30e817363e5590343d28639af921b2a5f1d40220010920ae4b00bbb657f8653cb44172b8cb13447bb5105ddaf32a2845ea0666b901 33 0x025ae98eff89505fa5ff60f919ae690de638d31f4f2fcab9a9deeaf4d48eda794b'
       }
     ],
     outputs: [
       {
         satoshis: 4294967000,
         script: '76a9143ec33076ba72b36b66b7ec571dd7417abdeb76f888ac'
       }
     ],
     nLockTime: 0
})

storage.updateTransaction(transaction);
```

