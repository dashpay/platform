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
  let nonDriveStorageFee = 0;
  let nonDriveProcessingFee = 0;

  let driveFeeResult;

  operations.forEach((operation) => {
    // Sum fees with RS Drive's Fee Result
    if (operation.feeResult && operation.feeResult.inner && operation.feeResult.feeRefunds.length > 0) {
      if (!driveFeeResult) {
        driveFeeResult = operation.feeResult;
      } else {
        driveFeeResult.add(operation.feeResult);
      }
    } else {
      nonDriveStorageFee += operation.getStorageCost();
      nonDriveProcessingFee += operation.getProcessingCost();
    }
  });

  if (driveFeeResult) {
    return {
      storageFee: driveFeeResult.storageFee + nonDriveStorageFee,
      processingFee: driveFeeResult.processingFee + nonDriveProcessingFee,
      feeRefunds: driveFeeResult.feeRefunds,
    };
  }

  return {
    storageFee: nonDriveStorageFee,
    processingFee: nonDriveProcessingFee,
    feeRefunds: [],
  };
}

module.exports = calculateOperationFees;
