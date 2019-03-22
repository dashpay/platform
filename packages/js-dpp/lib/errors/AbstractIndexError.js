const ConsensusError = require('./ConsensusError');

/**
 * @class
 * @abstract
 */
class AbstractIndexError extends ConsensusError {
  /**
   * @param {string} message
   * @param {rawContract} rawContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(message, rawContract, documentType, indexDefinition) {
    super(message);

    this.rawContract = rawContract;
    this.documentType = documentType;
    this.indexDefintion = indexDefinition;
  }

  /**
   * Get raw Contract
   *
   * @return {rawContract}
   */
  getRawContract() {
    return this.rawContract;
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
