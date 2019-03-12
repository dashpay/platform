const AbstractIndexError = require('./AbstractIndexError');

class UniqueIndexMustHaveUserIdPrefixError extends AbstractIndexError {
  /**
   * @param {rawDPContract} rawDPContract
   * @param {string} dpObjectType
   * @param {Object} indexDefinition
   */
  constructor(rawDPContract, dpObjectType, indexDefinition) {
    const message = 'Unique index must contain $userId as a first property';

    super(
      message,
      rawDPContract,
      dpObjectType,
      indexDefinition,
    );
  }
}

module.exports = UniqueIndexMustHaveUserIdPrefixError;
