const ValidationError = require('./ValidationError');

class InvalidPropertiesInOrderByError extends ValidationError {
  /**
   * @param {string} property
   */
  constructor(property) {
    super(`Invalid orderBy property ${property}. Should be used in a range "where" query`);
  }
}

module.exports = InvalidPropertiesInOrderByError;
