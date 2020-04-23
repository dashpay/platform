const TransactionEstimator = require('../../src/utils/coinSelections/TransactionEstimator.js');
const { sortAndVerifyUTXOS } = require('../../src/utils/coinSelections/helpers');

module.exports = function craftedGenerousMinerStrategy(utxosList, outputsList, deductFee = false, feeCategory = 'normal') {
  const txEstimator = new TransactionEstimator(feeCategory);

  // We add our outputs, theses will change only in case deductfee being true
  txEstimator.addOutputs(outputsList);

  const sort = [{ sortBy: 'satoshis', direction: 'descending' }];
  const sortedUtxosList = sortAndVerifyUTXOS(utxosList, sort);

  const totalOutputValue = txEstimator.getTotalOutputValue();

  let pendingSatoshis = 0;
  const simplyAccumulatedUtxos = sortedUtxosList.filter((utxo) => {
    if (pendingSatoshis < totalOutputValue) {
      pendingSatoshis += utxo.satoshis;
      return utxo;
    }
    return false;
  });
  if (pendingSatoshis < totalOutputValue) {
    throw new Error('Unsufficient utxo amount');
  }

  // We add the expected inputs, which should match the requested amount
  // TODO : handle case when we do not match it.
  txEstimator.addInputs(simplyAccumulatedUtxos);

  const estimatedFee = txEstimator.getFeeEstimate() + 10;
  if (deductFee === true) {
    // Then we check that we will be able to do it
    const inValue = txEstimator.getInValue();
    const outValue = txEstimator.getOutValue();
    if (inValue < outValue + estimatedFee) {
      // We don't have enough change for fee, so we remove from outValue
      txEstimator.reduceFeeFromOutput((outValue + estimatedFee) - inValue);
    } else {
      // TODO : Here we can add some process to check up that we clearly have enough to deduct fee
    }
  }

  return {
    utxos: txEstimator.getInputs(),
    outputs: txEstimator.getOutputs(),
    feeCategory,
    estimatedFee,
    utxosValue: txEstimator.getInValue(),
  };
};
