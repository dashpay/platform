const AbstractOperation = require('./AbstractOperation');

const DummyFeeResult = require('../DummyFeeResult');

class PreCalculatedOperation extends AbstractOperation {
  /**
   * @param {FeeResult|DummyFeeResult} feeResult
   */
  constructor(feeResult) {
    super();

    this.feeResult = feeResult;
  }

  /**
   * Get CPU cost of the operation
   *
   * @returns {number}
   */
  getProcessingCost() {
    return this.feeResult.processingFee;
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return this.feeResult.storageFee;
  }

  /**
   * @return {{identifier: Buffer, creditsPerEpoch: Object<string, number>}[]}
   */
  getRefunds() {
    return this.feeResult.feeRefunds;
  }

  /**
   * @return {{
   *  processingCost: number,
   *  type: string,
   *  storageCost: number,
   *  feeRefunds: {identifier: Buffer, creditsPerEpoch: Object<string, number>}[]
   * }}
   */
  toJSON() {
    return {
      type: 'preCalculated',
      storageCost: this.getStorageCost(),
      processingCost: this.getProcessingCost(),
      feeRefunds: this.getRefunds(),
    };
  }

  /**
   * @param {{
   *  storageCost: number,
   *  type: string,
   *  processingCost: number,
   *  feeRefunds: {identifier: Buffer, creditsPerEpoch: Object<string, number>}[]
   * }} json
   * @return {PreCalculatedOperation}
   */
  static fromJSON(json) {
    const staticFeeResult = new DummyFeeResult(
      json.storageCost,
      json.processingCost,
      json.feeRefunds,
    );

    return new PreCalculatedOperation(staticFeeResult);
  }
}

module.exports = PreCalculatedOperation;
