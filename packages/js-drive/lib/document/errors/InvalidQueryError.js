class InvalidQueryError extends Error {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid query: ${message}`);

    this.name = this.constructor.name;

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = InvalidQueryError;
