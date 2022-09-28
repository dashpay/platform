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

  const { storageFee, processingFee } = calculateOperationFees(
    executionContext.getOperations(),
  );

  // Is not implemented yet
  const storageRefund = 0;

  return (storageFee + processingFee) + DEFAULT_USER_TIP - storageRefund;
}

module.exports = calculateStateTransitionFee;
