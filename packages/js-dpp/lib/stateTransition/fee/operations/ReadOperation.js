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

  /**
   * @return {{valueSize: number, type: string}}
   */
  toJSON() {
    return {
      type: 'read',
      valueSize: this.valueSize,
    };
  }

  /**
   * @param {{type: string, valueSize: number}} json
   * @return {ReadOperation}
   */
  static fromJSON(json) {
    return new ReadOperation(json.valueSize);
  }
}

module.exports = ReadOperation;
