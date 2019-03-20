const AbstractIndexError = require('./AbstractIndexError');

class DuplicateIndexError extends AbstractIndexError {
  /**
   * @param {rawDPContract} rawDPContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(rawDPContract, documentType, indexDefinition) {
    const message = `Duplicate index definition for "${documentType}" document`;

    super(
      message,
      rawDPContract,
      documentType,
      indexDefinition,
    );
  }
}

module.exports = DuplicateIndexError;
