const AbstractIndexError = require('./AbstractIndexError');

class DuplicateIndexError extends AbstractIndexError {
  /**
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(rawDataContract, documentType, indexDefinition) {
    const message = `Duplicate index definition for "${documentType}" document`;

    super(
      message,
      rawDataContract,
      documentType,
      indexDefinition,
    );
  }
}

module.exports = DuplicateIndexError;
