const Operation = require("./Operation");

class PreCalculatedOperation extends Operation {
  /**
   * @param {number} cpuCost 
   * @param {number} storageCost 
   */
  constructor(cpuCost, storageCost) {
    this.cpuCost = cpuCost;
    this.storageCost = storageCost;
  }

  /**
   * Get CPU cost of the operation
   * 
   * @returns {number}
   */
   getCpuCost() {
    return this.cpuCost;
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
    return Operation.TYPES.PRE_CALCULATED;
  }
}

module.exports = PreCalculatedOperation;