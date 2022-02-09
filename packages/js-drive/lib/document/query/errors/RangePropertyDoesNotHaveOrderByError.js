const ValidationError = require('./ValidationError');

class RangePropertyDoesNotHaveOrderByError extends ValidationError {
  /**
   * @param {string} propertyName
   * @param {string} operator
   */
  constructor(propertyName, operator) {
    super(`Invalid range clause with '${propertyName}' and '${operator}' operator. Range operator must be used with order by`);
  }
}

module.exports = RangePropertyDoesNotHaveOrderByError;
