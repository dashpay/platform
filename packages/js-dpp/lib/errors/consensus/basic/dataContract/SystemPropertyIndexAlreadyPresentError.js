const AbstractIndexError = require('./AbstractIndexError');

class SystemPropertyIndexAlreadyPresentError extends AbstractIndexError {
  /**
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   */
  constructor(documentType, indexDefinition, propertyName) {
    const message = `Single-property index on ${propertyName} system property is already indexed for ${documentType} document.`;

    super(
      message,
      documentType,
      indexDefinition,
    );

    this.propertyName = propertyName;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
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

module.exports = SystemPropertyIndexAlreadyPresentError;
