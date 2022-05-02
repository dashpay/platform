const AbstractOperation = require('./AbstractOperation');

class ReadOperation extends AbstractOperation {
  /**
   * @param {number} valueSize
   */
  constructor(valueSize) {
    super();

    this.valueSize = valueSize;
  }

  /**
   * Get CPU cost of the operation
   *
   * @returns {number}
   */
  getProcessingCost() {
    return ReadOperation.BASE_PROCESSING_COST + this.valueSize * ReadOperation.CREDIT_PER_BYTE;
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return 0;
  }

  /**
   * Get operation type
   *
   * @returns {string}
   */
  getType() {
    return AbstractOperation.TYPES.READ;
  }
}

ReadOperation.CREDIT_PER_BYTE = 12;
ReadOperation.BASE_PROCESSING_COST = 8400;

module.exports = ReadOperation;
