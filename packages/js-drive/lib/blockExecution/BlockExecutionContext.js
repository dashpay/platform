const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const {
  tendermint: {
    abci: {
      LastCommitInfo,
    },
    types: {
      Header,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

class BlockExecutionContext {
  constructor() {
    this.dataContracts = [];
    this.cumulativeFees = 0;
    this.header = null;
    this.lastCommitInfo = null;
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
   * Set current block lastCommitInfo
   * @param {ILastCommitInfo} lastCommitInfo
   * @return {BlockExecutionContext}
   */
  setLastCommitInfo(lastCommitInfo) {
    this.lastCommitInfo = lastCommitInfo;

    return this;
  }

  /**
   * Get block lastCommitInfo
   *
   * @return {ILastCommitInfo|null}
   */
  getLastCommitInfo() {
    return this.lastCommitInfo;
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
    this.lastCommitInfo = null;
    this.validTxs = 0;
    this.invalidTxs = 0;
    this.consensusLogger = null;
  }

  /**
   * Check is the context is not set
   *
   * @return {boolean}
   */
  isEmpty() {
    return !this.header;
  }

  /**
   * Populate the current instance with data from another instance
   *
   * @param {BlockExecutionContext} blockExecutionContext
   */
  populate(blockExecutionContext) {
    this.dataContracts = blockExecutionContext.dataContracts;
    this.lastCommitInfo = blockExecutionContext.lastCommitInfo;
    this.cumulativeFees = blockExecutionContext.cumulativeFees;
    this.header = blockExecutionContext.header;
    this.validTxs = blockExecutionContext.validTxs;
    this.invalidTxs = blockExecutionContext.invalidTxs;
    this.consensusLogger = blockExecutionContext.consensusLogger;
  }

  /**
   * Populate the current instance with data
   *
   * @param object
   */
  fromObject(object) {
    this.dataContracts = object.dataContracts
      .map((rawDataContract) => new DataContract(rawDataContract));
    this.lastCommitInfo = LastCommitInfo.fromObject(object.lastCommitInfo);
    this.cumulativeFees = object.cumulativeFees;
    this.header = Header.fromObject(object.header);
    this.validTxs = object.validTxs;
    this.invalidTxs = object.invalidTxs;
    this.consensusLogger = object.consensusLogger;

    this.header.time.seconds = Long.fromNumber(this.header.time.seconds);
    this.header.height = Long.fromNumber(this.header.height);
  }

  /**
   * @param {Object} options
   * @param {boolean} [options.skipConsensusLogger=false]
   * @return {{
   *  dataContracts: Object[],
   *  invalidTxs: number,
   *  header: null,
   *  validTxs: number,
   *  cumulativeFees: number
   * }}
   */
  toObject(options = {}) {
    const header = Header.toObject(this.header);

    header.time.seconds = header.time.seconds.toNumber();
    header.height = header.height.toNumber();

    const object = {
      dataContracts: this.dataContracts.map((dataContract) => dataContract.toObject()),
      cumulativeFees: this.cumulativeFees,
      header,
      lastCommitInfo: LastCommitInfo.toObject(this.lastCommitInfo),
      validTxs: this.validTxs,
      invalidTxs: this.invalidTxs,
    };

    if (!options.skipConsensusLogger) {
      object.consensusLogger = this.consensusLogger;
    }

    return object;
  }
}

module.exports = BlockExecutionContext;
