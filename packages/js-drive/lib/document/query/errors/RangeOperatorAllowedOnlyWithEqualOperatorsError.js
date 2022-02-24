const ValidationError = require('./ValidationError');

class RangeOperatorAllowedOnlyWithEqualOperatorsError extends ValidationError {
  /**
   * @param {string} propertyName
   * @param {string} operator
   */
  constructor(propertyName, operator) {
    super(`Invalid range clause with '${propertyName}' and '${operator}' operator. Range operator are allowed with "=" or "in" operators`);
  }
}

module.exports = RangeOperatorAllowedOnlyWithEqualOperatorsError;
