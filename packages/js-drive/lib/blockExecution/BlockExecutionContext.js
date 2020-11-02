class BlockExecutionContext {
  constructor() {
    this.dataContracts = [];
    this.accumulativeFees = 0;
    this.header = null;
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
   * Check is data contract with specific ID is persistent in the context
   *
   * @param {Identifier} dataContractId
   * @return {boolean}
   */
  hasDataContract(dataContractId) {
    const index = this.dataContracts
      .findIndex((dataContract) => dataContractId.equals(dataContract.getId()));

    return index !== -1;
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
   * Set current block header
   * @param {IHeader} header
   * @return {BlockExecutionContext}
   */
  setHeader(header) {
    this.header = header;

    return this;
  }

  /**
   * Get block header
   *
   * @return {IHeader|null}
   */
  getHeader() {
    return this.header;
  }

  /**
   * Reset state
   */
  reset() {
    this.dataContracts = [];
    this.accumulativeFees = 0;
    this.header = null;
  }
}

module.exports = BlockExecutionContext;
