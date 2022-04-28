const Operation = require("./Operation");

class Read extends Operation {
  /**
   * @param {number} keySize 
   * @param {number} pathSize
   * @param {number} valueSize 
   */
   constructor(keySize, pathSize, valueSize) {
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
    return (this.keySize + this.pathSize + this.valueSize) * Operation.QUERY_CREDIT_PER_BYTE;
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
    return Operation.TYPES.READ;
  }
}

module.exports = Read;