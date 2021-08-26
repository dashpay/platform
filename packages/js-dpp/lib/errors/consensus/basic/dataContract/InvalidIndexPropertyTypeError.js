const AbstractIndexError = require('./AbstractIndexError');

class InvalidIndexPropertyTypeError extends AbstractIndexError {
  /**
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   * @param {string} propertyType
   */
  constructor(rawDataContract, documentType, indexDefinition, propertyName, propertyType) {
    const message = `'${propertyName}' property has an invalid type ${propertyType} and can not be used as index`;

    super(
      message,
      rawDataContract,
      documentType,
      indexDefinition,
    );

    this.propertyName = propertyName;
    this.propertyType = propertyType;
  }

  /**
   * Get property name
   *
   * @return {string}
   */
  getPropertyName() {
    return this.propertyName;
  }

  /**
   * Get property type name
   *
   * @return {string}
   */
  getPropertyType() {
    return this.propertyType;
  }
}

module.exports = InvalidIndexPropertyTypeError;
