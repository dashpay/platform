class RPCError extends Error {
  /**
   * @param {number} code
   * @param {string} message
   * @param {Object} [data]
   */
  constructor(code, message, data = undefined) {
    super();

    this.code = code;
    this.message = message;
    this.data = data;
    this.name = this.constructor.name;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get original error code
   *
   * @returns {number}
   */
  getCode() {
    return this.code;
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
