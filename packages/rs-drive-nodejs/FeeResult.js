// This file is crated when run `npm run build`. The actual source file that
// exports those functions is ./src/lib.rs
const {
  feeResultAdd,
  feeResultGetStorageFee,
  feeResultGetProcessingFee,
  feeResultAddFees,
  feeResultCreate,
  feeResultGetRefunds,
  feeResultSumRefundsPerEpoch,
} = require('neon-load-or-build')({
  dir: __dirname,
});

const { appendStack } = require('./appendStack');

const feeResultAddWithStack = appendStack(feeResultAdd);
const feeResultAddFeesWithStack = appendStack(feeResultAddFees);
const feeResultGetStorageFeeWithStack = appendStack(feeResultGetStorageFee);
const feeResultGetProcessingFeeWithStack = appendStack(feeResultGetProcessingFee);
const feeResultCreateWithStack = appendStack(feeResultCreate);
const feeResultGetRefundsWithStack = appendStack(feeResultGetRefunds);
const feeResultSumRefundsPerEpochWithStack = appendStack(feeResultSumRefundsPerEpoch);

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
   * Credit refunds
   *
   * @return {{identifier: Buffer, creditsPerEpoch: Object<string, number>}[]}
   */
  get feeRefunds() {
    return feeResultGetRefundsWithStack.call(this.inner);
  }

  /**
   * Sum credit refunds per epoch
   *
   * @returns {Object<string, number>}[]}
   */
  sumFeeRefundsPerEpoch() {
    return feeResultSumRefundsPerEpochWithStack.call(this.inner);
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
   * @param {BigInt} storageFee
   * @param {BigInt} processingFee
   * @param {FeeRefund[]} feeRefunds
   *
   * @returns {FeeResult}
   */
  static create(storageFee, processingFee, feeRefunds) {
    // TODO: until we use Neon version 1 we  can't really use BigInt
    const storageFeeBuffer = Buffer.allocUnsafe(8);
    storageFeeBuffer.writeBigInt64BE(storageFee, 0);

    const processingFeeBuffer = Buffer.allocUnsafe(8);
    processingFeeBuffer.writeBigInt64BE(processingFee, 0);

    const feeRefundsWithBuffers = feeRefunds.map((refund) => {
      const { identifier, creditsPerEpoch } = refund.toObject();

      const creditsPerEpochWithBuffers = {};
      creditsPerEpoch.forEach((value, key) => {
        const refundBuffer = Buffer.allocUnsafe(8);
        refundBuffer.writeBigInt64BE(value);

        creditsPerEpochWithBuffers[key] = refundBuffer;
      });

      return {
        identifier,
        creditsPerEpoch: creditsPerEpochWithBuffers,
      };
    });

    const inner = feeResultCreateWithStack(
      storageFeeBuffer,
      processingFeeBuffer,
      feeRefundsWithBuffers,
    );

    return new FeeResult(inner);
  }
}

module.exports = FeeResult;
