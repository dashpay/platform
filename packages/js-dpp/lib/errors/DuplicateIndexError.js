const AbstractIndexError = require('./AbstractIndexError');

class DuplicateIndexError extends AbstractIndexError {
  /**
   * @param {rawDPContract} rawDPContract
   * @param {string} dpObjectType
   * @param {Object} indexDefinition
   */
  constructor(rawDPContract, dpObjectType, indexDefinition) {
    const message = `Duplicate index definition for "${dpObjectType}" object`;

    super(
      message,
      rawDPContract,
      dpObjectType,
      indexDefinition,
    );
  }
}

module.exports = DuplicateIndexError;
