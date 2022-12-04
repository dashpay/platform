// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
  feeResultAdd,
  feeResultGetStorageFee,
  feeResultGetProcessingFee,
  feeResultAddFees,
} = require('neon-load-or-build')({
  dir: __dirname,
});

const { appendStack } = require('./appendStack');

const feeResultAddWithStack = appendStack(feeResultAdd);
const feeResultAddFeesWithStack = appendStack(feeResultAddFees);
const feeResultGetStorageFeeWithStack = appendStack(feeResultGetStorageFee);
const feeResultGetProcessingFeeWithStack = appendStack(feeResultGetProcessingFee);

class FeeResult {
  constructor(inner) {
    this.inner = inner;
  }

  /**
   * Processing fees
   *
   * @returns {number}
   */
  get processingFee() {
    return feeResultGetProcessingFeeWithStack.call(this.inner);
  }

  /**
   * Storage fees
   *
   * @returns {number}
   */
  get storageFee() {
    return feeResultGetStorageFeeWithStack.call(this.inner);
  }

  /**
   * Adds and self assigns result between two Fee Results
   *
   * @param {FeeResult} feeResult
   */
  add(feeResult) {
    this.inner = feeResultAddWithStack.call(this.inner, feeResult.inner);
  }

  /**
   * @param {number} storageFees
   * @param {number} processingFees
   */
  addFees(storageFees, processingFees) {
    feeResultAddFeesWithStack.call(this.inner, storageFees, processingFees);
  }
}

module.exports = FeeResult;
