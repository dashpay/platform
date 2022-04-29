const AbstractOperation = require('./AbstractOperation');

class PreCalculatedOperation extends AbstractOperation {
  /**
   * @param {number} storageCost
   * @param {number} processingCost
   */
  constructor(storageCost, processingCost) {
    super();

    this.storageCost = storageCost;
    this.processingCost = processingCost;
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
   * Get operation type
   *
   * @returns {string}
   */
  getType() {
    return AbstractOperation.TYPES.PRE_CALCULATED;
  }
}

module.exports = PreCalculatedOperation;
