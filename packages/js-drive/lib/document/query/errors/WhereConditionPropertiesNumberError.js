const ValidationError = require('./ValidationError');

class WhereConditionPropertiesNumberError extends ValidationError {
  /**
   * @param {number} numberOfProperties
   */
  constructor(numberOfProperties) {
    super(`"where" condition should have not less than ${numberOfProperties} (number of indexed properties minus 2) properties`);
  }
}

module.exports = WhereConditionPropertiesNumberError;
