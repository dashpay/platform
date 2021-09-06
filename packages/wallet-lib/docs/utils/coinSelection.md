**Usage**: `coinSelection(utxosList, outputsList, deductFee, feeCategory, strategy)`    
**Description**: For a provided outputsList will select the best utxos from utxosList matching the fees and strategy requirements 

Parameters: 

| parameters        | type          | required       | Description                                      |  
|-------------------|---------------|----------------| -------------------------------------------------|
| **utxosList**  | [UTXO]   | yes            | Account store with addresses                           |
| **outputsList**  | [Output]   | yes            | The account index                           |
| **deductFee**    | Boolean   | no (def: false)            | The wallet type                           |
| **feeCategory**    | FeeCategory   | no (def: normal)            | The wallet type                           |
| **strategy**    | Strategy   | no (def: simpleDescendingAccumulator)            | The wallet type                           |

Returns : {[ClassifiedAddresses]} - Array of classified addresses 

```js
coinSelection(utxosList, outputsList, true);

{
    utxos,
    outputs,
    feeCategory,
    estimatedFee,
    utxosValue,
  }
```