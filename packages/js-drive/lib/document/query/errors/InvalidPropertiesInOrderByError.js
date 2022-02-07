const ValidationError = require('./ValidationError');

class InvalidPropertiesInOrderByError extends ValidationError {
  /**
   * @param {string} property
   */
  constructor(property) {
    super(`${property} should be used in a range "where" query`);
  }
}

module.exports = InvalidPropertiesInOrderByError;
