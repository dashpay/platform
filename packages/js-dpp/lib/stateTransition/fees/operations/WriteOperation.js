const AbstractOperation = require('./AbstractOperation');

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
  getCpuCost() {
    return (this.keySize + this.valueSize) * AbstractOperation.STORAGE_PROCESSING_CREDIT_PER_BYTE;
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return (this.keySize + this.valueSize) * AbstractOperation.STORAGE_CREDIT_PER_BYTE;
  }

  /**
   * Get operation type
   *
   * @returns {string}
   */
  getType() {
    return AbstractOperation.TYPES.WRITE;
  }
}

module.exports = WriteOperation;
