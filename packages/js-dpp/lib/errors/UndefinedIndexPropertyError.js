const AbstractIndexError = require('./AbstractIndexError');

class UndefinedIndexPropertyError extends AbstractIndexError {
  /**
   * @param {rawDPContract} rawDPContract
   * @param {string} dpObjectType
   * @param {Object} indexDefinition
   * @param {string} propertyName
   */
  constructor(rawDPContract, dpObjectType, indexDefinition, propertyName) {
    const message = `Object does not have '${propertyName}' property`;

    super(
      message,
      rawDPContract,
      dpObjectType,
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
