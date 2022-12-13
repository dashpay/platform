const FeeResult = require('@dashevo/rs-drive/FeeResult');

/**
 * Calculate processing and storage fees based on operations
 *
 * @param {FeeResult[]} feeResults
 *
 * @returns {FeeResult}
 */
function aggregateFees(feeResults) {
  const rootFeeResult = FeeResult.create();

  feeResults.forEach((feeResult) => {
    rootFeeResult.add(feeResult);
  });

  return rootFeeResult;
}

module.exports = aggregateFees;
