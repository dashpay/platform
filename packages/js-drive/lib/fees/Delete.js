const Operation = require("./Operation");

class Delete extends Operation {
  /**
   * @param {number} keySize 
   * @param {number} valueSize 
   */
   constructor(keySize, valueSize) {
    this.keySize = keySize;
    this.valueSize = valueSize;
  }

  /**
   * Get CPU cost of the operation
   * 
   * @returns {number}
   */
  getCpuCost() {
    return (this.keySize + this.valueSize) * Operation.STORAGE_PROCESSING_CREDIT_PER_BYTE;
  }

  /**
   * Get storage cost of the operation
   * 
   * @returns {number}
   */
  getStorageCost() {
    return -((this.keySize + this.valueSize) * Operation.STORAGE_CREDIT_PER_BYTE);
  }
  
  /**
   * Get operation type
   * 
   * @returns {string}
   */
  getType() {
    return Operation.TYPES.DELETE;
  }
}

module.exports = Delete;