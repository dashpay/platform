/**
 * @param {calculateStateTransitionFeeFromOperations} calculateStateTransitionFeeFromOperations
 * @returns {calculateStateTransitionFee}
 */
function calculateStateTransitionFeeFactory(calculateStateTransitionFeeFromOperations) {
  /**
   * @typedef {calculateStateTransitionFee}
   * @param {AbstractStateTransition} stateTransition
   * @return {{
   *   storageFee: number,
   *   processingFee: number,
   *   feeRefunds: {identifier: Buffer, creditsPerEpoch: Object<string, number>}[],
   *   totalRefunds: number,
   *   total: number,
   *   desiredAmount: number,
   *   requiredAmount: number,
   * }}
   */
  function calculateStateTransitionFee(stateTransition) {
    const executionContext = stateTransition.getExecutionContext();

    return calculateStateTransitionFeeFromOperations(
      executionContext.getOperations(),
      stateTransition.getOwnerId(),
    );
  }

  return calculateStateTransitionFee;
}

module.exports = calculateStateTransitionFeeFactory;
