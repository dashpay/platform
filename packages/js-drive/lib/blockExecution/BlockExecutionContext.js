const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const {
  tendermint: {
    abci: {
      CommitInfo,
    },
    version: {
      Consensus,
    },
  },
  google: {
    protobuf: {
      Timestamp,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

class BlockExecutionContext {
  constructor() {
    this.init();
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
  getCumulativeStorageFee() {
    return this.cumulativeStorageFee;
  }

  /**
   * @return {number}
   */
  getCumulativeProcessingFee() {
    return this.cumulativeProcessingFee;
  }

  /**
   * Increment cumulative fees
   *
   * @param {number} fee
   */
  incrementCumulativeStorageFee(fee) {
    this.cumulativeStorageFee += fee;

    return this;
  }

  /**
   * Increment cumulative fees
   *
   * @param {number} fee
   */
  incrementCumulativeProcessingFee(fee) {
    this.cumulativeProcessingFee += fee;

    return this;
  }

  /**
   *
   * @param {number} coreChainLockedHeight
   * @return {BlockExecutionContext}
   */
  setCoreChainLockedHeight(coreChainLockedHeight) {
    this.coreChainLockedHeight = coreChainLockedHeight;

    return this;
  }

  /**
   *
   * @return {number}
   */
  getCoreChainLockedHeight() {
    return this.coreChainLockedHeight;
  }

  /**
   * @param {Long} height
   * @return {BlockExecutionContext}
   */
  setHeight(height) {
    this.height = height;

    return this;
  }

  /**
   *
   * @return {Long}
   */
  getHeight() {
    return this.height;
  }

  /**
   *
   * @param {IConsensus} version
   * @return {BlockExecutionContext}
   */
  setVersion(version) {
    this.version = version;

    return this;
  }

  /**
   *
   * @return {IConsensus}
   */
  getVersion() {
    return this.version;
  }

  /**
   *
   * @param {ITimestamp} time
   * @return {BlockExecutionContext}
   */
  setTime(time) {
    this.time = time;

    return this;
  }

  /**
   *
   * @return {ITimestamp}
   */
  getTime() {
    return this.time;
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
   * Set withdrawal transactions hash map
   *
   * @param {Object} withdrawalTransactionsMap
   *
   * @returns {BlockExecutionContext}
   */
  setWithdrawalTransactionsMap(withdrawalTransactionsMap) {
    this.withdrawalTransactionsMap = withdrawalTransactionsMap;

    return this;
  }

  /**
   * Get withdrawal transactions hash map
   *
   * @return {Object}
   */
  getWithdrawalTransactionsMap() {
    return this.withdrawalTransactionsMap;
  }

  /**
   * @param {Long} height
   * @return {BlockExecutionContext}
   */
  setPreviousHeight(height) {
    this.previousHeight = height;

    return this;
  }

  /**
   *
   * @return {Long}
   */
  getPreviousHeight() {
    return this.previousHeight;
  }

  /**
   *
   * @param {ITimestamp} time
   * @return {BlockExecutionContext}
   */
  setPreviousTime(time) {
    this.previousBlockTime = time;

    return this;
  }

  /**
   *
   * @return {ITimestamp}
   */
  getPreviousTime() {
    return this.previousBlockTime;
  }

  /**
   *
   * @param {number} coreChainLockedHeight
   * @return {BlockExecutionContext}
   */
  setPreviousCoreChainLockedHeight(coreChainLockedHeight) {
    this.previousCoreChainLockedHeight = coreChainLockedHeight;

    return this;
  }

  /**
   *
   * @return {number}
   */
  getPreviousCoreChainLockedHeight() {
    return this.previousCoreChainLockedHeight;
  }

  /**
   * @private
   *
   * init store
   */
  init() {
    this.previousBlockTime = null;
    this.previousHeight = null;
    this.previousCoreChainLockedHeight = null;

    this.reset();
  }

  /**
   * Reset state
   */
  reset() {
    this.dataContracts = [];
    this.cumulativeProcessingFee = 0;
    this.cumulativeStorageFee = 0;
    this.coreChainLockedHeight = null;
    this.height = null;
    this.version = null;
    this.time = null;
    this.lastCommitInfo = null;
    this.validTxs = 0;
    this.invalidTxs = 0;
    this.consensusLogger = null;
    this.withdrawalTransactionsMap = {};
  }

  /**
   * Check is the context is not set
   *
   * @return {boolean}
   */
  isEmpty() {
    return this.height === null;
  }

  /**
   * Populate the current instance with data from another instance
   *
   * @param {BlockExecutionContext} blockExecutionContext
   */
  populate(blockExecutionContext) {
    this.dataContracts = blockExecutionContext.dataContracts;
    this.lastCommitInfo = blockExecutionContext.lastCommitInfo;
    this.cumulativeProcessingFee = blockExecutionContext.cumulativeProcessingFee;
    this.cumulativeStorageFee = blockExecutionContext.cumulativeStorageFee;
    this.time = blockExecutionContext.time;
    this.height = blockExecutionContext.height;
    this.coreChainLockedHeight = blockExecutionContext.coreChainLockedHeight;
    this.version = blockExecutionContext.version;
    this.validTxs = blockExecutionContext.validTxs;
    this.invalidTxs = blockExecutionContext.invalidTxs;
    this.consensusLogger = blockExecutionContext.consensusLogger;
    this.previousBlockTime = blockExecutionContext.previousBlockTime;
    this.previousHeight = blockExecutionContext.previousHeight;
    this.previousCoreChainLockedHeight = blockExecutionContext.previousCoreChainLockedHeight;
    this.withdrawalTransactionsMap = blockExecutionContext.withdrawalTransactionsMap;
  }

  /**
   * Populate the current instance with data
   *
   * @param {Object} object
   */
  fromObject(object) {
    this.dataContracts = object.dataContracts
      .map((rawDataContract) => new DataContract(rawDataContract));
    this.lastCommitInfo = CommitInfo.fromObject(object.lastCommitInfo);
    this.cumulativeProcessingFee = object.cumulativeProcessingFee;
    this.cumulativeStorageFee = object.cumulativeStorageFee;
    this.validTxs = object.validTxs;
    this.invalidTxs = object.invalidTxs;
    this.consensusLogger = object.consensusLogger;

    if (object.time) {
      this.time = new Timestamp({
        seconds: Long.fromNumber(object.time.seconds),
      });
    }

    this.height = Long.fromNumber(object.height);
    this.coreChainLockedHeight = object.coreChainLockedHeight;
    this.version = Consensus.fromObject(object.version);

    this.previousBlockTime = new Timestamp({
      seconds: Long.fromNumber(object.previousBlockTime.seconds),
    });

    this.previousHeight = Long.fromNumber(object.previousHeight);
    this.previousCoreChainLockedHeight = object.previousCoreChainLockedHeight;
    this.withdrawalTransactionsMap = object.withdrawalTransactionsMap;
  }

  /**
   * @param {Object} options
   * @param {boolean} [options.skipConsensusLogger=false]
   * @return {{
   *  dataContracts: Object[],
   *  invalidTxs: number,
   *  height: number,
   *  version: Object,
   *  time: Object,
   *  validTxs: number,
   *  cumulativeProcessingFee: number,
   *  cumulativeStorageFee: number,
   *  coreChainLockedHeight: number,
   *  lastCommitInfo: number,
   *  previousBlockTime: number,
   *  previousHeight: number,
   *  previousCoreChainLockedHeight: number,
   *  withdrawalTransactionsMap: Object,
   * }}
   */
  toObject(options = {}) {
    let time = null;

    if (this.time) {
      time = this.time.toJSON();
      time.seconds = Number(time.seconds);
    }

    const object = {
      dataContracts: this.dataContracts.map((dataContract) => dataContract.toObject()),
      cumulativeProcessingFee: this.cumulativeProcessingFee,
      cumulativeStorageFee: this.cumulativeStorageFee,
      time,
      height: this.height ? this.height.toNumber() : null,
      version: this.version ? this.version.toJSON() : null,
      coreChainLockedHeight: this.coreChainLockedHeight,
      lastCommitInfo: CommitInfo.toObject(this.lastCommitInfo),
      validTxs: this.validTxs,
      invalidTxs: this.invalidTxs,
      previousBlockTime: this.previousBlockTime,
      previousHeight: this.previousHeight,
      previousCoreChainLockedHeight: this.previousCoreChainLockedHeight,
      withdrawalTransactionsMap: this.withdrawalTransactionsMap,
    };

    if (!options.skipConsensusLogger) {
      object.consensusLogger = this.consensusLogger;
    }

    return object;
  }
}

module.exports = BlockExecutionContext;
