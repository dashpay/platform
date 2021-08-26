const AbstractIndexError = require('./AbstractIndexError');

class InvalidCompoundIndexError extends AbstractIndexError {
  /**
   *
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(rawDataContract, documentType, indexDefinition) {
    super(
      'All or none of unique compound index properties must be set',
      rawDataContract,
      documentType,
      indexDefinition,
    );
  }
}

module.exports = InvalidCompoundIndexError;
