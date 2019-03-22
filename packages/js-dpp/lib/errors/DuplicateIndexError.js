const AbstractIndexError = require('./AbstractIndexError');

class DuplicateIndexError extends AbstractIndexError {
  /**
   * @param {rawContract} rawContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(rawContract, documentType, indexDefinition) {
    const message = `Duplicate index definition for "${documentType}" document`;

    super(
      message,
      rawContract,
      documentType,
      indexDefinition,
    );
  }
}

module.exports = DuplicateIndexError;
