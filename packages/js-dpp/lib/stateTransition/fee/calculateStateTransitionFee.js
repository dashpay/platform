const {
  DEFAULT_USER_TIP,
} = require('./constants');

const calculateOperationFees = require('./calculateOperationFees');

/**
 * @typedef {calculateStateTransitionFee}
 * @param {AbstractStateTransition} stateTransition
 * @return {number}
 */
function calculateStateTransitionFee(stateTransition) {
  const executionContext = stateTransition.getExecutionContext();

  const calculatedFees = calculateOperationFees(executionContext.getOperations());

  const {
    storageFee,
    processingFee,
    feeRefunds,
  } = calculatedFees;

  if (feeRefunds.length > 1) {
    throw new Error('State Transition removed bytes from several identities that is not defined by protocol');
  }

  let feeRefundsSum = 0;
  if (feeRefunds.length > 0) {
    const stateTransitionIdentifier = stateTransition.getOwnerId();

    if (!stateTransitionIdentifier.equals(feeRefunds[0].identifier)) {
      throw new Error('State Transition removed bytes from different identity');
    }

    // TODO: We should deduct leftovers

    feeRefundsSum = feeRefunds[0].creditsPerEpoch.entries()
      .reduce((sum, [, credits]) => sum + credits, 0);
  }

  // Fee refunds are negative
  const total = (storageFee + processingFee + feeRefundsSum) + DEFAULT_USER_TIP;

  executionContext.setLastCalculatedFeeDetails({ ...calculatedFees, feeRefundsSum, total });

  return total;
}

module.exports = calculateStateTransitionFee;
