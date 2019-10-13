const AbstractIndexError = require('./AbstractIndexError');

class UniqueIndexMustHaveUserIdPrefixError extends AbstractIndexError {
  /**
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(rawDataContract, documentType, indexDefinition) {
    const message = 'Unique index must contain $userId as a first property';

    super(
      message,
      rawDataContract,
      documentType,
      indexDefinition,
    );
  }
}

module.exports = UniqueIndexMustHaveUserIdPrefixError;
