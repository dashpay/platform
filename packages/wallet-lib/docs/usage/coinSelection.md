## Coin Selection

### Get a coin selection

The coin Selection helpers will take several mendatory parameters.

- utxosList
- outputsLst
- strategyName (optional - default : 'simpleAccumulator')

```
const utxosList = account.getUTXOS();
const outputsList = [{
    address:'XmjeE...',
    satoshis:1200000
}]
const coinSelection = coinSelection(utxosList, outputsList);
const selectedUTXO = coinSelection(utxosList, outputsList);
```
