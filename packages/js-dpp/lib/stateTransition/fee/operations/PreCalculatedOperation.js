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
   * @return {{processingCost: number, type: string, storageCost: number}}
   */
  toJSON() {
    return {
      type: 'preCalculated',
      storageCost: this.getStorageCost(),
      processingCost: this.getProcessingCost(),
    };
  }

  /**
   * @param {{storageCost: number, type: string, processingCost: number}} json
   * @return {PreCalculatedOperation}
   */
  static fromJSON(json) {
    const staticFeeResult = new DummyFeeResult(json.storageCost, json.processingCost);

    return new PreCalculatedOperation(staticFeeResult);
  }
}

module.exports = PreCalculatedOperation;
