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

    if (feeRefunds.length > 1) {
      throw new Error('State Transition removed bytes from several identities that is not defined by protocol');
    }

    let feeRefundsSum = 0;
    if (feeRefunds.length > 0) {
      const stateTransitionIdentifier = stateTransition.getOwnerId();

      if (!stateTransitionIdentifier.equals(feeRefunds[0].identifier)) {
        throw new Error('State Transition removed bytes from different identity');
      }

      feeRefundsSum = await Object.entries(feeRefunds[0].creditsPerEpoch)
        .reduce(async (sum, [epochIndex, credits]) => {
          const [amount, leftovers] = await stateRepository
            .calculateStorageFeeDistributionAmountAndLeftovers(credits, Number(epochIndex));

          return (await sum) + amount + leftovers;
        }, 0);
    }

    // TODO: we should prepay for balance update after ST execution

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
