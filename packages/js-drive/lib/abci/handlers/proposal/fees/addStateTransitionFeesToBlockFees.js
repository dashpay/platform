/**
 * @param {BlockFees} blockFees
 * @param {BlockFees} stFees
 */
function addStateTransitionFeesToBlockFees(blockFees, stFees) {
  /* eslint-disable no-param-reassign */
  blockFees.storageFee += stFees.storageFee;
  blockFees.processingFee += stFees.processingFee;

  for (const [epochIndex, credits] of Object.entries(stFees.refundsPerEpoch)) {
    if (!blockFees.refundsPerEpoch[epochIndex]) {
      blockFees.refundsPerEpoch[epochIndex] = 0;
    }

    blockFees.refundsPerEpoch[epochIndex] += credits;
  }

  return blockFees;
}

module.exports = addStateTransitionFeesToBlockFees;
