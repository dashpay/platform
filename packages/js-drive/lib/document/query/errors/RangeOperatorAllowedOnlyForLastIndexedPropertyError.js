const ValidationError = require('./ValidationError');

class RangeOperatorAllowedOnlyForLastIndexedPropertyError extends ValidationError {
  /**
   * @param {string} propertyName
   * @param {string} operator
   */
  constructor(propertyName, operator) {
    super(`Invalid range clause with '${propertyName}' and '${operator}' operator. Range operator are allowed only for the last indexed property`);
  }
}

module.exports = RangeOperatorAllowedOnlyForLastIndexedPropertyError;
