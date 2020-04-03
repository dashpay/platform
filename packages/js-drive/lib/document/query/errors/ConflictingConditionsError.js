const ValidationError = require('./ValidationError');

class ConflictingConditionsError extends ValidationError {
  /**
   * @param {string} field
   * @param {string[]} operators
   */
  constructor(field, operators) {
    super(`Using multiple conditions (${operators.join(', ')})`
      + ` with a single field ("${field}") is not allowed`);

    this.field = field;
    this.operators = operators;
  }

  /**
   * Get field name
   *
   * @return {string}
   */
  getField() {
    return this.field;
  }

  /**
   * Get operators
   *
   * @return {string[]}
   */
  getOperators() {
    return this.operators;
  }
}

module.exports = ConflictingConditionsError;
