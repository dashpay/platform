class BlockExecutionState {
  constructor() {
    this.dataContracts = [];
  }

  /**
   * Add Data Contract
   *
   * @param {DataContract|null} dataContract
   */
  addDataContract(dataContract) {
    this.dataContracts.push(dataContract);
  }

  /**
   * Get Data Contracts
   *
   * @returns {DataContract[]}
   */
  getDataContracts() {
    return this.dataContracts;
  }

  /**
   * Reset state
   */
  reset() {
    this.dataContracts = [];
  }
}

module.exports = BlockExecutionState;
