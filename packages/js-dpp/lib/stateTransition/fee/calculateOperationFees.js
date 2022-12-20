const {
  FEE_MULTIPLIER,
} = require('./constants');

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

  let feeResult;

  operations.forEach((operation) => {
    // TODO should use checked add when moved to Rust

    storageFee += operation.getStorageCost();
    processingFee += operation.getProcessingCost();

    // Combine refunds which are currently present only in RS Drive's Fee Result
    if (operation.feeResult && operation.feeResult.inner) {
      if (!feeResult) {
        feeResult = operation.feeResult;
      } else {
        feeResult.add(operation.feeResult);
      }
    }
  });

  // TODO: Do we need to multiply pre calculated fees?

  storageFee *= FEE_MULTIPLIER;
  processingFee *= FEE_MULTIPLIER;

  return {
    storageFee,
    processingFee,
    feeRefunds: feeResult ? feeResult.feeRefunds : [],
  };
}

module.exports = calculateOperationFees;
