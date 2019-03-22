const AbstractIndexError = require('./AbstractIndexError');

class UndefinedIndexPropertyError extends AbstractIndexError {
  /**
   * @param {RawContract} rawContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   */
  constructor(rawContract, documentType, indexDefinition, propertyName) {
    const message = `'${propertyName}' property is not defined in the document`;

    super(
      message,
      rawContract,
      documentType,
      indexDefinition,
    );

    this.propertyName = propertyName;
  }

  /**
   * Get property name
   *
   * @return {string}
   */
  getPropertyName() {
    return this.propertyName;
  }
}

module.exports = UndefinedIndexPropertyError;
