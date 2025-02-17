class ErrorResult {
  /**
   * @param {number} code
   * @param {string} message
   * @param {Buffer|undefined} data
   */
  constructor(code, message, data) {
    this.code = code;
    this.message = message;
    this.data = data;
  }

  /**
   * @returns {number}
   */
  getCode() {
    return this.code;
  }

  /**
   * @returns {string}
   */
  getMessage() {
    return this.message;
  }

  /**
   * @returns {Buffer|undefined}
   */
  getData() {
    return this.data;
  }
}

module.exports = ErrorResult;
