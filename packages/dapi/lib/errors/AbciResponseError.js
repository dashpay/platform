class AbciResponseError extends Error {
  /**
   * @param {number} errorCode
   * @param {Object} abciError
   * @param {string} abciError.message
   * @param {Object} abciError.data
   */
  constructor(errorCode, { message, data }) {
    super();

    this.errorCode = errorCode;
    this.message = message;
    this.data = data;
  }

  /**
   * Get error code
   *
   * @return {number}
   */
  getErrorCode() {
    return this.errorCode;
  }

  /**
   * Get error message
   *
   * @return {string}
   */
  getMessage() {
    return this.message;
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

module.exports = AbciResponseError;
