const {
  FEE_MULTIPLIER,
} = require('@dashevo/dpp/lib/stateTransition/fee/constants');

const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/PreCalculatedOperation');
const FeeResult = require('@dashevo/rs-drive/FeeResult');

/**
 * Calculate processing and storage fees based on operations
 *
 * @param {AbstractOperation[]} operations
 *
 * @returns {FeeResult}
 */
function aggregateOperationFees(operations) {
  let storageFee = 0;
  let processingFee = 0;

  const rootFeeResult = FeeResult.create();

  // TODO: All operations must have the same interface (must based on FeeResult?)
  operations.forEach((operation) => {
    if (operation instanceof PreCalculatedOperation) {
      // Add all subsequent ones to the root fee result
      rootFeeResult.add(operation.feeResult);
    } else {
      // Collect fees from operation which are not based on FeeResult
      storageFee += operation.getProcessingCost();
      processingFee += operation.getStorageCost();
    }
  });

  // TODO: Do we need to multiply pre calculated fees?

  storageFee *= FEE_MULTIPLIER;
  processingFee *= FEE_MULTIPLIER;

  rootFeeResult.addFees(storageFee, processingFee);

  return rootFeeResult;
}

module.exports = aggregateOperationFees;
