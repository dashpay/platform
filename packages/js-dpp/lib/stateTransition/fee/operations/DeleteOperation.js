const AbstractOperation = require('./AbstractOperation');

const {
  DELETE_BASE_PROCESSING_COST,
  PROCESSING_CREDIT_PER_BYTE,
  STORAGE_CREDIT_PER_BYTE,
} = require('../constants');

class DeleteOperation extends AbstractOperation {
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
      + DELETE_BASE_PROCESSING_COST;
  }

  /**
   * Get storage cost of the operation
   *
   * @returns {number}
   */
  getStorageCost() {
    return -((this.keySize + this.valueSize) * STORAGE_CREDIT_PER_BYTE);
  }

  /**
   * @return {{keySize: number, type: string, valueSize: number}}
   */
  toJSON() {
    return {
      type: 'delete',
      keySize: this.keySize,
      valueSize: this.valueSize,
    };
  }

  /**
   * @param {{keySize: number, type: string, valueSize: number}} json
   * @return {DeleteOperation}
   */
  static fromJSON(json) {
    return new DeleteOperation(json.keySize, json.valueSize);
  }
}

module.exports = DeleteOperation;
