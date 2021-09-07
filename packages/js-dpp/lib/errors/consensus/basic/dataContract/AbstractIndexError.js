const AbstractBasicError = require('../AbstractBasicError');

/**
 * @class
 * @abstract
 */
class AbstractIndexError extends AbstractBasicError {
  /**
   * @param {string} message
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(message, documentType, indexDefinition) {
    super(message);

    this.documentType = documentType;
    this.indexDefintion = indexDefinition;
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
