class AbciError extends Error {
  /**
   * @param {number} code
   * @param {string} message
   * @param {Object=undefined} [data]
   * @param {Object=} [tags]
   */
  constructor(code, message, data = undefined, tags = {}) {
    super();

    this.name = this.constructor.name;

    this.code = code;
    this.message = message;
    this.data = data;
    this.tags = tags;

    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Get message
   *
   * @return {string}
   */
  getMessage() {
    return this.message;
  }

  /**
   * Get error code
   *
   * @return {number}
   */
  getCode() {
    return this.code;
  }

  /**
   * Get data
   *
   * @return {Object}
   */
  getData() {
    return this.data;
  }

  /**
   * Get tags
   *
   * @return {Object}
   */
  getTags() {
    return this.tags;
  }
}

AbciError.CODES = {
  INTERNAL: 1,
  INVALID_ARGUMENT: 2,
  NOT_FOUND: 3,
  INSUFFICIENT_FUNDS: 4,
  EXECUTION_TIMED_OUT: 5,
  MEMORY_LIMIT_EXCEEDED: 6,
  UNAVAILABLE: 7,
};

module.exports = AbciError;
