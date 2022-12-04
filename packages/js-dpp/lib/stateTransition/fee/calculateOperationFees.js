const {
  FEE_MULTIPLIER,
} = require('./constants');

const PreCalculatedOperation = require('./operations/PreCalculatedOperation');

/**
 * Calculate processing and storage fees based on operations
 *
 * @param {AbstractOperation[]} operations
 *
 * @returns {FeeResult}
 */
function calculateOperationFees(operations) {
  let storageFee = 0;
  let processingFee = 0;

  let rootFeeResult;

  // TODO: All operations must have the same interface (must based on FeeResult?)
  operations.forEach((operation) => {
    if (operation instanceof PreCalculatedOperation) {
      if (!rootFeeResult) {
        // Use first fee result as a root one
        rootFeeResult = operation.feeResult;
      } else {
        // Add all subsequent ones to the root fee result
        rootFeeResult.add(operation.feeResult);
      }
    } else {
      // Collect fees from operation which are not based on FeeResult
      storageFee += operation.getProcessingCost();
      processingFee += operation.getStorageCost();
    }
  });

  storageFee *= FEE_MULTIPLIER;
  processingFee *= FEE_MULTIPLIER;

  rootFeeResult.addFees(storageFee, processingFee);

  return rootFeeResult;
}

module.exports = calculateOperationFees;
