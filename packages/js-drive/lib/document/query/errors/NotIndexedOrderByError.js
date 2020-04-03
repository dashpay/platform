const ValidationError = require('./ValidationError');

class NotIndexedOrderByError extends ValidationError {
  /**
   *
   * @param {string} field
   */
  constructor(field) {
    super(`Order by not indexed field "${field}" is not allowed`);

    this.field = field;
  }

  /**
   * Get not indexed field
   *
   * @returns {string}
   */
  getNotIndexedField() {
    return this.field;
  }
}

module.exports = NotIndexedOrderByError;
