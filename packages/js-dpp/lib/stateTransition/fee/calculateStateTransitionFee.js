const calculateOperationCosts = require('./calculateOperationCosts');

const {
  FEE_MULTIPLIER,
  DEFAULT_USER_TIP,
} = require('./constants');

/**
 * @typedef {calculateStateTransitionFee}
 * @param {AbstractStateTransition} stateTransition
 * @return {number}
 */
function calculateStateTransitionFee(stateTransition) {
  const executionContext = stateTransition.getExecutionContext();

  const { storageCost, processingCost } = calculateOperationCosts(
    executionContext.getOperations(),
  );

  // Is not implemented yet
  const storageRefund = 0;

  return (storageCost + processingCost) * FEE_MULTIPLIER + DEFAULT_USER_TIP - storageRefund;
}

module.exports = calculateStateTransitionFee;
