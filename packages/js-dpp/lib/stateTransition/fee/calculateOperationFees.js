const {
  FEE_MULTIPLIER,
} = require('./constants');

/**
 * Calculate processing and storage fees based on operations
 *
 * @param {AbstractOperation[]} operations
 *
 * @returns {{ storageFee: number, processingFee: number }}
 */
function calculateOperationFees(operations) {
  const fees = {
    storageFee: 0,
    processingFee: 0,
  };

  operations.forEach((operation) => {
    fees.storageFee += operation.getProcessingCost();
    fees.processingFee += operation.getStorageCost();
  });

  fees.storageFee *= FEE_MULTIPLIER;
  fees.processingFee *= FEE_MULTIPLIER;

  return fees;
}

module.exports = calculateOperationFees;
