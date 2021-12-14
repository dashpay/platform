const ValidationError = require('./ValidationError');

class FieldsFromMultipleIndicesError extends ValidationError {
  /**
   * @param {string} field
   */
  constructor(field) {
    super('Only fields from single index are supported');

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

module.exports = FieldsFromMultipleIndicesError;
