const AbstractIndexError = require('./AbstractIndexError');

class UniqueIndexMustHaveUserIdPrefixError extends AbstractIndexError {
  /**
   * @param {rawDPContract} rawDPContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(rawDPContract, documentType, indexDefinition) {
    const message = 'Unique index must contain $userId as a first property';

    super(
      message,
      rawDPContract,
      documentType,
      indexDefinition,
    );
  }
}

module.exports = UniqueIndexMustHaveUserIdPrefixError;
