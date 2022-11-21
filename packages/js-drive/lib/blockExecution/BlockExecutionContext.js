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
    this.reset();
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
   * Set committed round
   *
   * @param {number} round
   *
   * @returns {BlockExecutionContext}
   */
  setRound(round) {
    this.round = round;

    return this;
  }

  /**
   * Get committed round
   *
   * @return {number}
   */
  getRound() {
    return this.round;
  }

  /**
   * Reset state
   */
  reset() {
    this.dataContracts = [];
    this.coreChainLockedHeight = null;
    this.height = null;
    this.version = null;
    this.time = null;
    this.lastCommitInfo = null;
    this.consensusLogger = null;
    this.withdrawalTransactionsMap = {};
    this.round = null;
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
    this.time = blockExecutionContext.time;
    this.height = blockExecutionContext.height;
    this.coreChainLockedHeight = blockExecutionContext.coreChainLockedHeight;
    this.version = blockExecutionContext.version;
    this.consensusLogger = blockExecutionContext.consensusLogger;
    this.withdrawalTransactionsMap = blockExecutionContext.withdrawalTransactionsMap;
    this.round = blockExecutionContext.round;
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
    this.consensusLogger = object.consensusLogger;

    if (object.time) {
      this.time = new Timestamp({
        seconds: Long.fromNumber(object.time.seconds),
      });
    }

    this.height = Long.fromNumber(object.height);
    this.coreChainLockedHeight = object.coreChainLockedHeight;
    this.version = Consensus.fromObject(object.version);
    this.withdrawalTransactionsMap = object.withdrawalTransactionsMap;
    this.round = object.round;
  }

  /**
   * @param {Object} options
   * @param {boolean} [options.skipConsensusLogger=false]
   * @return {{
   *  dataContracts: Object[],
   *  height: number,
   *  version: Object,
   *  time: Object,
   *  coreChainLockedHeight: number,
   *  lastCommitInfo: number,
   *  previousBlockTime: number,
   *  previousHeight: number,
   *  previousCoreChainLockedHeight: number,
   *  withdrawalTransactionsMap: Object,
   *  round: number,
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
      time,
      height: this.height ? this.height.toNumber() : null,
      version: this.version ? this.version.toJSON() : null,
      coreChainLockedHeight: this.coreChainLockedHeight,
      lastCommitInfo: this.lastCommitInfo ? CommitInfo.toObject(this.lastCommitInfo) : null,
      withdrawalTransactionsMap: this.withdrawalTransactionsMap,
      round: this.round,
    };

    if (!options.skipConsensusLogger) {
      object.consensusLogger = this.consensusLogger;
    }

    return object;
  }
}

module.exports = BlockExecutionContext;
