const AbstractOperation = require('./AbstractOperation');

class PreCalculatedOperation extends AbstractOperation {
  /**
   * @param {number} storageCost
   * @param {number} processingCost
   */
  constructor(storageCost, processingCost) {
    super();

    this.storageCost = storageCost || 0;
    this.processingCost = processingCost || 0;
  }

  /**
   * Get CPU cost of the operation
   *
   * @returns {number}
   */
  getProcessingCost() {
    return this.processingCost;
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return this.storageCost;
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
    return new PreCalculatedOperation(json.storageCost, json.processingCost);
  }
}

module.exports = PreCalculatedOperation;
