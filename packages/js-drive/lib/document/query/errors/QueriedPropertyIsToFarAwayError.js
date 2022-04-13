const ValidationError = require('./ValidationError');

class QueriedPropertyIsToFarAwayError extends ValidationError {
  /**
   * @param {string} property
   */
  constructor(property) {
    super(`${property} property is used in a range query and is more than 2 properties away from last indexed one`);
  }
}

module.exports = QueriedPropertyIsToFarAwayError;
