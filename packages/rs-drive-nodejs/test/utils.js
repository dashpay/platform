const { expect } = require('chai');

/**
 * @param {FeeResult} feeResult
 */
function expectFeeResult(feeResult) {
  expect(feeResult).to.have.property('processingFee');
  expect(feeResult).to.have.property('storageFee');
  expect(feeResult).to.have.property('removedFromIdentities');

  expect(feeResult.processingFee).to.be.greaterThan(0, 'processing fee must be higher than 0');
  expect(feeResult.storageFee).to.be.greaterThan(0, 'storage fee must be higher than 0');
}

module.exports = {
  expectFeeResult,
};
