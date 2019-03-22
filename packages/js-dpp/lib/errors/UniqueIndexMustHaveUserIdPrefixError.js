const AbstractIndexError = require('./AbstractIndexError');

class UniqueIndexMustHaveUserIdPrefixError extends AbstractIndexError {
  /**
   * @param {rawContract} rawContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(rawContract, documentType, indexDefinition) {
    const message = 'Unique index must contain $userId as a first property';

    super(
      message,
      rawContract,
      documentType,
      indexDefinition,
    );
  }
}

module.exports = UniqueIndexMustHaveUserIdPrefixError;
