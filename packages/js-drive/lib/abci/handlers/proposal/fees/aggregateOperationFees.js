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

  operations.forEach((operation) => {
    if (operation instanceof PreCalculatedOperation && operation.feeResult instanceof FeeResult) {
      rootFeeResult.add(operation.feeResult);
    } else {
      // Collect fees from operation which are not based on FeeResult
      // and add to the root fee result later
      storageFee += operation.getStorageCost();
      processingFee += operation.getProcessingCost();
    }
  });

  // TODO: Do we need to multiply pre calculated fees?

  storageFee *= FEE_MULTIPLIER;
  processingFee *= FEE_MULTIPLIER;

  rootFeeResult.addFees(storageFee, processingFee);

  return rootFeeResult;
}

module.exports = aggregateOperationFees;
