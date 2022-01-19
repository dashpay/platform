const ValidationError = require('./ValidationError');

class NotLastOperatorInWhereConditionsError extends ValidationError {
  /**
   * @param {string} operator
   */
  constructor(operator) {
    super(`Where condition with '${operator}' operator should be last in the list`);
  }
}

module.exports = NotLastOperatorInWhereConditionsError;
