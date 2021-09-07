const AbstractIndexError = require('./AbstractIndexError');

class DuplicateIndexError extends AbstractIndexError {
  /**
   * @param {string} documentType
   * @param {Object} indexDefinition
   */
  constructor(documentType, indexDefinition) {
    const message = `Duplicate index definition for "${documentType}" document`;

    super(
      message,
      documentType,
      indexDefinition,
    );

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = DuplicateIndexError;
