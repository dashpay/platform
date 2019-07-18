const ValidationError = require('./ValidationError');

class NestedSystemFieldError extends ValidationError {
  /**
   * @param {string} field
   */
  constructor(field) {
    super('Nested "elementMatch" operator is not supported');

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
