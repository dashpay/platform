class RPCError extends Error {
  /**
   * @param {string} message
   * @param {Object} [data]
   */
  constructor(message, data = undefined) {
    super();

    this.message = message;
    this.data = data;
    this.name = this.constructor.name;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get error data
   *
   * @return {Object}
   */
  getData() {
    return this.data;
  }
}

module.exports = RPCError;
