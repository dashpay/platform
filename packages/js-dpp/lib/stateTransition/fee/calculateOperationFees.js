/**
 * Calculate processing and storage fees based on operations
 *
 *
 * @typedef {calculateOperationFees}
 * @param {AbstractOperation[]} operations
 *
 * @returns {{
 *   storageFee: number,
 *   processingFee: number,
 *   feeRefunds: {identifier: Buffer, creditsPerEpoch: Object<string, number>}[]
 * }}
 */
function calculateOperationFees(operations) {
  let storageFee = 0;
  let processingFee = 0;
  const feeRefunds = [];

  operations.forEach((operation) => {
    storageFee += operation.getStorageCost();
    processingFee += operation.getProcessingCost();

    // Merge refunds
    const operationRefunds = operation.getRefunds();

    operationRefunds.forEach((identityRefunds) => {
      const existingIdentityRefunds = feeRefunds
        .find(({ identifier }) => identifier.equals(identityRefunds.identifier));

      if (!existingIdentityRefunds) {
        feeRefunds.push(identityRefunds);

        return;
      }

      Object.entries(identityRefunds.creditsPerEpoch).forEach(([epochIndex, credits]) => {
        if (!existingIdentityRefunds.creditsPerEpoch[epochIndex]) {
          existingIdentityRefunds.refundsPerEpoch[epochIndex] = 0;
        }

        existingIdentityRefunds.refundsPerEpoch[epochIndex] += credits;
      });
    });
  });

  return {
    storageFee,
    processingFee,
    feeRefunds,
  };
}

module.exports = calculateOperationFees;
