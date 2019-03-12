const ConsensusError = require('./ConsensusError');

/**
 * @class
 * @abstract
 */
class AbstractIndexError extends ConsensusError {
  /**
   * @param {string} message
   * @param {rawDPContract} rawDPContract
   * @param {string} dpObjectType
   * @param {Object} indexDefinition
   */
  constructor(message, rawDPContract, dpObjectType, indexDefinition) {
    super(message);

    this.rawDPContract = rawDPContract;
    this.dpObjectType = dpObjectType;
    this.indexDefintion = indexDefinition;
  }

  /**
   * Get raw DP Contract
   *
   * @return {rawDPContract}
   */
  getRawDPContract() {
    return this.rawDPContract;
  }

  /**
   * Get DP Object type
   *
   * @return {Object}
   */
  getDPObjectType() {
    return this.dpObjectType;
  }

  /**
   * Get index definition
   *
   * @return {Object}
   */
  getIndexDefinition() {
    return this.indexDefintion;
  }
}

module.exports = AbstractIndexError;
