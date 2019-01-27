class InvalidParamsError extends Error {
  /**
   * @param {string} message
   * @param {Object|array} [data]
   */
  constructor(message, data = undefined) {
    super();

    this.name = this.constructor.name;
    this.message = message;
    this.data = data;

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = InvalidParamsError;
