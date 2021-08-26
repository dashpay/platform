const AbstractIndexError = require('./AbstractIndexError');

class InvalidIndexedPropertyConstraintError extends AbstractIndexError {
  /**
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   * @param {string} constraintName
   * @param {string} reason
   */
  constructor(
    rawDataContract,
    documentType,
    indexDefinition,
    propertyName,
    constraintName,
    reason,
  ) {
    const message = `Indexed property '${propertyName}' have invalid constraint '${constraintName}',`
      + ` reason '${reason}'`;

    super(
      message,
      rawDataContract,
      documentType,
      indexDefinition,
    );

    this.propertyName = propertyName;
    this.constraintName = constraintName;
    this.reason = reason;
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
