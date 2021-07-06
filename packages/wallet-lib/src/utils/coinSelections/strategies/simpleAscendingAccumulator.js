const { sortBy } = require('lodash');
const TransactionEstimator = require('../TransactionEstimator');
/**
 * Given a utxos list and a threesholdSatoshis, will add them
 * without any further logic up to met with requested params.
 * @param utxos
 * @param thresholdSatoshis
 * @return {*}
 */
const simplyAccumulateUtxos = (utxos, thresholdSatoshis) => {
  let pendingSatoshis = 0;
  const accumulatedUtxos = utxos.filter((utxo) => {
    if (pendingSatoshis < thresholdSatoshis) {
      pendingSatoshis += utxo.satoshis;
      return utxo;
    }
    return false;
  });
  if (pendingSatoshis < thresholdSatoshis) {
    throw new Error('Unsufficient utxo amount');
  }
  return accumulatedUtxos;
};
/**
 * Simple accumulator strategy : Will try to spend using as few utxos as possible.
 * Sorted by ascending value amount.
 * @param {*} utxosList - A utxos list
 * @param {*} outputsList - The output list
 * @param {*} deductFee - default: false - Deduct fee from outputs
 * @param {*} feeCategory - default: normal

 */

// FIXME : If we have a dust, it might cost us more to spend it (fee) than we earn.
// Might want to not spend it
const simpleAscendingAccumulator = (utxosList, outputsList, deductFee = false, feeCategory = 'normal') => {
  const txEstimator = new TransactionEstimator(feeCategory);

  // We add our outputs, theses will change only in case deductfee being true
  txEstimator.addOutputs(outputsList);

  const sortedUtxosList = sortBy(utxosList, ['-satoshis', 'txid', 'outputIndex']);

  const totalOutputValue = txEstimator.getTotalOutputValue();
  const simplyAccumulatedUtxos = simplyAccumulateUtxos(sortedUtxosList, totalOutputValue);

  // We add the expected inputs, which should match the requested amount
  // TODO : handle case when we do not match it.
  txEstimator.addInputs(simplyAccumulatedUtxos);

  const estimatedFee = txEstimator.getFeeEstimate();
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
  // console.log('estimatedFee are', estimatedFee, 'satoshis');
  return {
    utxos: txEstimator.getInputs(),
    outputs: txEstimator.getOutputs(),
    feeCategory,
    estimatedFee,
    utxosValue: txEstimator.getInValue(),
  };
};
module.exports = simpleAscendingAccumulator;
