# Coin Selection

## Purpose 

In order to decide which set of input to use for making payments, wallet-lib use the coin selection helper which will decide which unspent transaction output (UTXO) to select.  

## Strategies

There are multiples strategy algorithms provided with Wallet-lib, that you can chose from. 

| Strategy                      | Description                                                                                                                                                                   |  
|-------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| simpleDescendingAccumulator   | Will maximize the uses of big inputs to meet the amount required. Allows the fee to be optimized for the smallest size at the cost of breaking big inputs.                    |
| simpleAscendingAccumulator    | Will try to use as many small inputs as possible to meet the amount required. Allows using many small inputs at the cost of a potentially bigger fee.                         |

By default, the algorithm that is being used is `simpleDescendingAccumulator`. 

See [Account.createTransaction()](/account/createTransaction) for more information about how to select one during transaction creation.  

Additionally, you can also require the utility function `const coinSelection = require('@dashevo/wallet-lib/src/utils/coinSelection.js')` for your own usage.  


```
const utxosList = account.getUTXOS();
const outputsList = [{
    address:'XmjeE...',
    satoshis:1200000
}]
const coinSelection = coinSelection(utxosList, outputsList);
const selectedUTXO = coinSelection(utxosList, outputsList);
```

## Implement your own algorithm

By creating a simple function algorithm that you pass to the createTransaction parameter, you can provide your own algorithm that will be used to the coinSelection.   

To implements your own algorithm, you might want to take example on the [already existing one](https://github.com/dashevo/wallet-lib/tree/master/src/utils/coinSelections/strategies).  
You will need your algorithm to handle multiples parameter : 

- `utxosList` - An array consisting of multiple [unspent output](https://github.com/dashevo/dashcore-lib/blob/master/docs/unspentoutput.md).
- `outputsList` - An array consisting of multiple [Output](https://github.com/dashevo/dashcore-lib/blob/master/docs/transaction.md#handling-outputs).
- `deductFee` - A simple boolean that indicates if we want to deduct fee from our outputs. (Can be useful for a control on how much we wish to spend at maximum).
- `feeCategory` - A simple enum of the fee category (normal, slow, fast,...).

Your algorithm will be required to return the following object structure : 

- `utxos`: An array consisting of the final selection of UTXOs.
- `outputs`: An array consisting of the final outputs (which might have been modified in case of deductFee being `true`).
- `estimatedFee`: A duff value of the fee estimated for such transaction.
- `utxosValue`: The total accumulated duffs value of the used UTXOs.
- `feeCategory`
