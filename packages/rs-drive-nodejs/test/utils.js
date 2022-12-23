const { expect } = require('chai');
const FeeResult = require('../FeeResult');

/**
 * @param {FeeResult} feeResult
 */
function expectFeeResult(feeResult) {
  expect(feeResult).to.be.an.instanceOf(FeeResult);

  expect(feeResult.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
  expect(feeResult.storageFee).to.be.greaterThan(0, 'storage fee must be higher than 0');
}

module.exports = {
  expectFeeResult,
};
