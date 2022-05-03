const AbstractOperation = require('./AbstractOperation');

const {
  READ_BASE_PROCESSING_COST,
  PROCESSING_CREDIT_PER_BYTE,
} = require('../constants');

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
    return READ_BASE_PROCESSING_COST + this.valueSize * PROCESSING_CREDIT_PER_BYTE;
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return 0;
  }
}

module.exports = ReadOperation;
