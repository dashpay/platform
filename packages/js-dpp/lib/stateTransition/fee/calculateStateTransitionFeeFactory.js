const {
  DEFAULT_USER_TIP,
} = require('./constants');

/**
 * @param {StateRepository} stateRepository
 * @param {calculateOperationFees} calculateOperationFees
 * @returns {calculateStateTransitionFee}
 */
function calculateStateTransitionFeeFactory(stateRepository, calculateOperationFees) {
  /**
   * @typedef {calculateStateTransitionFee}
   * @param {AbstractStateTransition} stateTransition
   * @return {Promise<number>}
   */
  async function calculateStateTransitionFee(stateTransition) {
    const executionContext = stateTransition.getExecutionContext();

    const calculatedFees = calculateOperationFees(executionContext.getOperations());

    const {
      storageFee,
      processingFee,
      feeRefunds,
    } = calculatedFees;

    const creditsPerEpoch = feeRefunds
      .find((refunds) => stateTransition.getOwnerId().equals(refunds.identifier));

    let feeRefundsSum = 0;

    if (creditsPerEpoch) {
      feeRefundsSum = await Object.entries(creditsPerEpoch)
        .reduce((sum, [, credits]) => sum + credits, 0);
    }

    // TODO: we need to introduce base fee for ST that includes balance update

    const requiredAmount = (storageFee - feeRefundsSum) + DEFAULT_USER_TIP;
    const desiredAmount = (storageFee + processingFee - feeRefundsSum) + DEFAULT_USER_TIP;

    executionContext.setLastCalculatedFeeDetails({
      ...calculatedFees,
      feeRefundsSum,
      requiredAmount,
      desiredAmount,
    });

    return desiredAmount;
  }

  return calculateStateTransitionFee;
}

module.exports = calculateStateTransitionFeeFactory;
