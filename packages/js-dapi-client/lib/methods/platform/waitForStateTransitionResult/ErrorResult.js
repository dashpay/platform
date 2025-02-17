class ErrorResult {
  /**
   * @param {number} code
   * @param {string} message
   * @param {*} data
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
   * @returns {*}
   */
  getData() {
    return this.data;
  }
}

module.exports = ErrorResult;
