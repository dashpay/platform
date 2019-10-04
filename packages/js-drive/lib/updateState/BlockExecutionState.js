/**
 * Store contracts during block execution process
 */
class BlockExecutionState {
  constructor() {
    this.contracts = [];
  }

  /**
   * Add new contract
   * @param {Contract|null} contract
   */
  addContract(contract) {
    if (contract !== null) {
      this.contracts.push(contract);
    }
  }

  /**
   * Get all contracts
   * @returns {Array}
   */
  getContracts() {
    return this.contracts;
  }

  /**
   * Clear all contracts after block was committed
   */
  clearContracts() {
    this.contracts = [];
  }
}

module.exports = BlockExecutionState;
