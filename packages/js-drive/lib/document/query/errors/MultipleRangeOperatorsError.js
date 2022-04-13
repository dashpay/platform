const ValidationError = require('./ValidationError');

class MultipleRangeOperatorsError extends ValidationError {
  /**
   * @param {string} propertyName
   * @param {string} operator
   */
  constructor(propertyName, operator) {
    super(`Invalid range clause with '${propertyName}' and '${operator}' operator. Only one range operator is allowed`);
  }
}

module.exports = MultipleRangeOperatorsError;
