const {
  DEFAULT_USER_TIP,
} = require('./constants');

/**
 * @param {calculateOperationFees} calculateOperationFees
 * @returns {calculateStateTransitionFee}
 */
function calculateStateTransitionFeeFactory(calculateOperationFees) {
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

    const ownerRefunds = feeRefunds
      .find((refunds) => stateTransition.getOwnerId().equals(refunds.identifier));

    let totalRefunds = 0;

    if (ownerRefunds) {
      totalRefunds = Object.entries(ownerRefunds.creditsPerEpoch)
        .reduce((sum, [, credits]) => sum + credits, 0);
    }

    // TODO: we need to introduce base fee for ST that includes balance update

    const requiredAmount = (storageFee - totalRefunds) + DEFAULT_USER_TIP;
    const desiredAmount = (storageFee + processingFee - totalRefunds) + DEFAULT_USER_TIP;

    // TODO: Do we really need this?
    executionContext.setLastCalculatedFeeDetails({
      ...calculatedFees,
      totalRefunds,
      requiredAmount,
      desiredAmount,
    });

    return desiredAmount;
  }

  return calculateStateTransitionFee;
}

module.exports = calculateStateTransitionFeeFactory;
