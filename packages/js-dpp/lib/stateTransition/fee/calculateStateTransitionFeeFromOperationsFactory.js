const { DEFAULT_USER_TIP } = require('./constants');

/**
 * @param {calculateOperationFees} calculateOperationFees
 * @returns {calculateStateTransitionFeeFromOperations}
 */
function calculateStateTransitionFeeFromOperationsFactory(
  calculateOperationFees,
) {
  /**
   * @typedef {calculateStateTransitionFeeFromOperations}
   * @param {AbstractOperation[]} operations
   * @param {Identifier} identityId
   * @returns {{
   *   storageFee: number,
   *   processingFee: number,
   *   feeRefunds: {identifier: Buffer, creditsPerEpoch: Object<string, number>}[],
   *   totalRefunds: number,
   *   total: number,
   *   desiredAmount: number,
   *   requiredAmount: number,
   * }}
   */
  function calculateStateTransitionFeeFromOperations(operations, identityId) {
    const calculatedFees = calculateOperationFees(operations);

    const {
      storageFee,
      processingFee,
      feeRefunds,
    } = calculatedFees;

    const ownerRefunds = feeRefunds
      .find((refunds) => identityId.equals(refunds.identifier));

    let totalRefunds = 0;

    if (ownerRefunds) {
      totalRefunds = Object.entries(ownerRefunds.creditsPerEpoch)
        .reduce((sum, [, credits]) => sum + credits, 0);
    }

    // TODO: we need to introduce base fee for ST that includes balance update

    const requiredAmount = (storageFee - totalRefunds) + DEFAULT_USER_TIP;
    const desiredAmount = (storageFee + processingFee - totalRefunds) + DEFAULT_USER_TIP;

    return {
      ...calculatedFees,
      totalRefunds,
      requiredAmount,
      desiredAmount,
    };
  }

  return calculateStateTransitionFeeFromOperations;
}

module.exports = calculateStateTransitionFeeFromOperationsFactory;
