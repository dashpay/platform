const AbstractIndexError = require('./AbstractIndexError');

class InvalidIndexedPropertyConstraintError extends AbstractIndexError {
  /**
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   * @param {string} constraintName
   * @param {string} reason
   */
  constructor(
    documentType,
    indexDefinition,
    propertyName,
    constraintName,
    reason,
  ) {
    const message = `Indexed property '${propertyName}' for ${documentType} document have invalid constraint '${constraintName}',`
      + ` reason '${reason}'`;

    super(
      message,
      documentType,
      indexDefinition,
    );

    this.propertyName = propertyName;
    this.constraintName = constraintName;
    this.reason = reason;

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
   * Get property constraint name
   *
   * @return {string}
   */
  getConstraintName() {
    return this.constraintName;
  }

  /**
   * Get invalidity reason
   *
   * @return {string}
   */
  getReason() {
    return this.reason;
  }
}

module.exports = InvalidIndexedPropertyConstraintError;
