const AbstractIndexError = require('./AbstractIndexError');

class SystemPropertyIndexAlreadyPresentError extends AbstractIndexError {
  /**
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   */
  constructor(rawDataContract, documentType, indexDefinition, propertyName) {
    const message = `Single-property index on ${propertyName} system property is already present for quering.`;

    super(
      message,
      rawDataContract,
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

module.exports = SystemPropertyIndexAlreadyPresentError;
