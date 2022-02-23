const ValidationError = require('./ValidationError');

class InOperatorAllowedOnlyForLastTwoIndexedPropertiesError extends ValidationError {
  /**
   * @param {string} propertyName
   * @param {string} operator
   */
  constructor(propertyName, operator) {
    super(`Invalid range clause with '${propertyName}' and '${operator}' operator. "in" operator are allowed only for the last two indexed properties`);
  }
}

module.exports = InOperatorAllowedOnlyForLastTwoIndexedPropertiesError;
