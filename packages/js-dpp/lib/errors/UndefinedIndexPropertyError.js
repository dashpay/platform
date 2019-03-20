const AbstractIndexError = require('./AbstractIndexError');

class UndefinedIndexPropertyError extends AbstractIndexError {
  /**
   * @param {rawDPContract} rawDPContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   */
  constructor(rawDPContract, documentType, indexDefinition, propertyName) {
    const message = `'${propertyName}' property is not defined in the document`;

    super(
      message,
      rawDPContract,
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
