// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
  feeResultAdd,
  feeResultGetStorageFee,
  feeResultGetProcessingFee,
  feeResultAddFees,
  feeResultCreate,
} = require('neon-load-or-build')({
  dir: __dirname,
});

const { appendStack } = require('./appendStack');

const feeResultAddWithStack = appendStack(feeResultAdd);
const feeResultAddFeesWithStack = appendStack(feeResultAddFees);
const feeResultGetStorageFeeWithStack = appendStack(feeResultGetStorageFee);
const feeResultGetProcessingFeeWithStack = appendStack(feeResultGetProcessingFee);
const feeResultCreateWithStack = appendStack(feeResultCreate);

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
   * @param {number} storageFee
   * @param {number} processingFee
   */
  addFees(storageFee, processingFee) {
    feeResultAddFeesWithStack.call(this.inner, storageFee, processingFee);
  }

  /**
   * Create new fee result
   *
   * @returns {FeeResult}
   */
  static create(storageFee = 0, processingFee = 0) {
    const inner = feeResultCreateWithStack(storageFee, processingFee);

    return new FeeResult(inner);
  }
}

module.exports = FeeResult;
