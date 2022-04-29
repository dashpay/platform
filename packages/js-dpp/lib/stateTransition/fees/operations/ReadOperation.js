const AbstractOperation = require('./AbstractOperation');

class ReadOperation extends AbstractOperation {
  /**
   * @param {number} keySize
   * @param {number} pathSize
   * @param {number} valueSize
   */
  constructor(keySize, pathSize, valueSize) {
    super();

    this.keySize = keySize;
    this.pathSize = pathSize;
    this.valueSize = valueSize;
  }

  /**
   * Get CPU cost of the operation
   *
   * @returns {number}
   */
  getCpuCost() {
    return (this.keySize + this.pathSize + this.valueSize)
      * AbstractOperation.QUERY_CREDIT_PER_BYTE;
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

module.exports = ReadOperation;
