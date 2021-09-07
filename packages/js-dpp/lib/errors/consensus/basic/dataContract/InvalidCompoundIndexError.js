const AbstractIndexError = require('./AbstractIndexError');

class InvalidCompoundIndexError extends AbstractIndexError {
  /**
   *
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(documentType, indexDefinition) {
    super(
      `All or none of unique compound index properties must be set for "${documentType}" document`,
      documentType,
      indexDefinition,
    );

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = InvalidCompoundIndexError;
