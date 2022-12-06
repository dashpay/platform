const {
  FEE_MULTIPLIER,
} = require('./constants');

/**
 * Calculate processing and storage fees based on operations
 *
 * @param {AbstractOperation[]} operations
 *
 * @returns {{storageFee: number, processingFee: number}}
 */
function calculateOperationFees(operations) {
  let storageFee = 0;
  let processingFee = 0;

  operations.forEach((operation) => {
    storageFee += operation.getProcessingCost();
    processingFee += operation.getStorageCost();
  });

  // TODO: Do we need to multiply pre calculated fees?

  storageFee *= FEE_MULTIPLIER;
  processingFee *= FEE_MULTIPLIER;

  return {
    storageFee,
    processingFee,
  };
}

module.exports = calculateOperationFees;
