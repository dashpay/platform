const is = require('../is');
const STRATEGIES = require('./strategies');
const InvalidUTXO = require('../../errors/InvalidUTXO');
const InvalidOutput = require('../../errors/InvalidOutput');
const CoinSelectionUnsufficientUTXOS = require('../../errors/CoinSelectionUnsufficientUTXOS');

module.exports = function coinSelection(utxosList, outputsList, deductFee = false, feeCategory = 'normal', strategy = STRATEGIES.simpleTransactionOptimizedAccumulator) {
  if (!utxosList) { throw new Error('A utxosList is required'); }
  if (utxosList.constructor.name !== Array.name) { throw new Error('UtxosList is expected to be an array of utxos'); }
  if (utxosList.length < 1) { throw new Error('utxosList must contain at least 1 utxo'); }
  let utxosValue = 0;


  for (let i = 0; i < utxosList.length; i += 1) {
    const utxo = utxosList[i];
    if (!is.utxo(utxo)) {
      throw new InvalidUTXO(utxo);
    }
    utxosValue += utxo.satoshis;
  }

  if (!outputsList) { throw new Error('An outputsList is required in order to perform a selection'); }
  if (outputsList.constructor.name !== Array.name) { throw new Error('outputsList must be an array of outputs'); }
  if (outputsList.length < 1) { throw new Error('outputsList must contains at least 1 output'); }

  let outputValue = 0;
  outputsList.forEach((output) => {
    if (!is.output(output)) {
      throw new InvalidOutput(output);
    }
    outputValue += output.satoshis;
  });
  if (utxosValue < outputValue) {
    throw new CoinSelectionUnsufficientUTXOS({ utxosValue, outputValue });
  }
  return strategy(utxosList, outputsList, deductFee, feeCategory);
};
