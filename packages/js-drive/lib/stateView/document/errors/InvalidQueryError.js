class InvalidQueryError extends Error {
  /**
   * @param {ValidationError[]} errors
   */
  constructor(errors) {
    super('Invalid query');

    this.name = this.constructor.name;

    Error.captureStackTrace(this, this.constructor);

    this.errors = errors;
  }

  /**
   * Get validation errors
   *
   * @return {ValidationError[]}
   */
  getErrors() {
    return this.errors;
  }
}

module.exports = InvalidQueryError;
