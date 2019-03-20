const ConsensusError = require('./ConsensusError');

/**
 * @class
 * @abstract
 */
class AbstractIndexError extends ConsensusError {
  /**
   * @param {string} message
   * @param {rawDPContract} rawDPContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(message, rawDPContract, documentType, indexDefinition) {
    super(message);

    this.rawDPContract = rawDPContract;
    this.documentType = documentType;
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
   * Get Document type
   *
   * @return {Object}
   */
  getDocumentType() {
    return this.documentType;
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
