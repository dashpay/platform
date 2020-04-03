class BlockExecutionState {
  constructor() {
    this.dataContracts = [];
    this.accumulativeFees = 0;
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
   * @return {number}
   */
  getAccumulativeFees() {
    return this.accumulativeFees;
  }

  /**
   * Increment accumulative fees
   *
   * @param {number} fee
   */
  incrementAccumulativeFees(fee) {
    this.accumulativeFees += fee;

    return this;
  }

  /**
   * Reset state
   */
  reset() {
    this.dataContracts = [];
    this.accumulativeFees = 0;
  }
}

module.exports = BlockExecutionState;
