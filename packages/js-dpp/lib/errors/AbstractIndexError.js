const ConsensusError = require('./ConsensusError');

/**
 * @class
 * @abstract
 */
class AbstractIndexError extends ConsensusError {
  /**
   * @param {string} message
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(message, rawDataContract, documentType, indexDefinition) {
    super(message);

    this.rawDataContract = rawDataContract;
    this.documentType = documentType;
    this.indexDefintion = indexDefinition;
  }

  /**
   * Get raw Data Contract
   *
   * @return {RawDataContract}
   */
  getRawDataContract() {
    return this.rawDataContract;
  }

  /**
   * Get Document type
   *
   * @return {string}
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
