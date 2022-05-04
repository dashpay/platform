const AbstractOperation = require('./AbstractOperation');

const {
  WRITE_BASE_PROCESSING_COST,
  PROCESSING_CREDIT_PER_BYTE,
  STORAGE_CREDIT_PER_BYTE,
} = require('../constants');

class WriteOperation extends AbstractOperation {
  /**
   * @param {number} keySize
   * @param {number} valueSize
   */
  constructor(keySize, valueSize) {
    super();

    this.keySize = keySize;
    this.valueSize = valueSize;
  }

  /**
   * Get CPU cost of the operation
   *
   * @returns {number}
   */
  getProcessingCost() {
    return ((this.keySize + this.valueSize) * PROCESSING_CREDIT_PER_BYTE)
      + WRITE_BASE_PROCESSING_COST;
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return (this.keySize + this.valueSize) * STORAGE_CREDIT_PER_BYTE;
  }

  /**
   * @return {{valueSize: number, type: string, keySize: number}}
   */
  toJSON() {
    return {
      type: 'write',
      keySize: this.keySize,
      valueSize: this.valueSize,
    };
  }

  /**
   * @param {{keySize: number, type: string, valueSize: number}} json
   * @return {WriteOperation}
   */
  static fromJSON(json) {
    return new WriteOperation(json.keySize, json.valueSize);
  }
}

module.exports = WriteOperation;
