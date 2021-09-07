const AbstractIndexError = require('./AbstractIndexError');

class InvalidIndexPropertyTypeError extends AbstractIndexError {
  /**
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   * @param {string} propertyType
   */
  constructor(documentType, indexDefinition, propertyName, propertyType) {
    const message = `'${propertyName}' property for ${documentType} document has an invalid type ${propertyType} and can not be used as index`;

    super(
      message,
      documentType,
      indexDefinition,
    );

    this.propertyName = propertyName;
    this.propertyType = propertyType;

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
