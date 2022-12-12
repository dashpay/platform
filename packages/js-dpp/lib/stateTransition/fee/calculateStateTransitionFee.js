const {
  DEFAULT_USER_TIP,
} = require('./constants');

const calculateOperationFees = require('./calculateOperationFees');

/**
 * @typedef {calculateStateTransitionFee}
 * @param {AbstractStateTransition} stateTransition
 * @param {Object} options
 * @param {boolean} [options.useCache=false]
 * @return {number}
 */
function calculateStateTransitionFee(stateTransition, options = {}) {
  const executionContext = stateTransition.getExecutionContext();

  if (options.useCache) {
    const calculatedFeeDetails = executionContext.getLastCalculatedFeeDetails();

    if (!calculatedFeeDetails) {
      throw new Error('State Transition Execution context doesn\'t contain cached fee calculation');
    }

    return calculatedFeeDetails.total;
  }

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

    feeRefundsSum = feeRefunds[0].creditsPerEpoch.entries()
      .reduce((sum, [, credits]) => sum + credits, 0);
  }

  const total = (storageFee + processingFee - feeRefundsSum) + DEFAULT_USER_TIP;

  executionContext.setLastCalculatedFeeDetails({ ...calculatedFees, feeRefundsSum, total });

  return total;
}

module.exports = calculateStateTransitionFee;
