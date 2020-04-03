const ValidationError = require('./ValidationError');

class DuplicateSortingFieldError extends ValidationError {
  /**
   * @param {string} field
   */
  constructor(field) {
    super(`Duplicate sorting field ${field}`);

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

module.exports = DuplicateSortingFieldError;
