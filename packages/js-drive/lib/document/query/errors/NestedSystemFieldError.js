const ValidationError = require('./ValidationError');

class NestedSystemFieldError extends ValidationError {
  /**
   * @param {string} field
   */
  constructor(field) {
    super(`Field ${field} is not supported in nested objects`);

    this.field = field;
  }

  /**
   * Get field name
   *
   * @return {string}
   */
  getField() {
    return this.field;
  }
}

module.exports = NestedSystemFieldError;
