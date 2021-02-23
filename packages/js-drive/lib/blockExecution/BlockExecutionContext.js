class BlockExecutionContext {
  constructor() {
    this.dataContracts = [];
    this.cumulativeFees = 0;
    this.header = null;
    this.validTxs = 0;
    this.invalidTxs = 0;
    this.consensusLogger = null;
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
  getCumulativeFees() {
    return this.cumulativeFees;
  }

  /**
   * Increment cumulative fees
   *
   * @param {number} fee
   */
  incrementCumulativeFees(fee) {
    this.cumulativeFees += fee;

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
   * Increment number of valid txs processed
   *
   * @return {BlockExecutionContext}
   */
  incrementValidTxCount() {
    this.validTxs += 1;

    return this;
  }

  /**
   * Increment number of invalid txs processed
   *
   * @return {BlockExecutionContext}
   */
  incrementInvalidTxCount() {
    this.invalidTxs += 1;

    return this;
  }

  /**
   * Get number of valid txs processed
   *
   * @return {number}
   */
  getValidTxCount() {
    return this.validTxs;
  }

  /**
   * Get number of invalid txs processed
   *
   * @return {number}
   */
  getInvalidTxCount() {
    return this.invalidTxs;
  }

  /**
   * Set consensus logger
   *
   * @param {BaseLogger} logger
   */
  setConsensusLogger(logger) {
    this.consensusLogger = logger;
  }

  /**
   * Get consensus logger
   *
   * @return {BaseLogger}
   */
  getConsensusLogger() {
    if (!this.consensusLogger) {
      throw new Error('Consensus logger has not been set');
    }

    return this.consensusLogger;
  }

  /**
   * Reset state
   */
  reset() {
    this.dataContracts = [];
    this.cumulativeFees = 0;
    this.header = null;
    this.validTxs = 0;
    this.invalidTxs = 0;
    this.consensusLogger = null;
  }
}

module.exports = BlockExecutionContext;
