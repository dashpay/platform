class ErrorResult {
  /**
   * @param {number} code
   * @param {string} message
   * @param {Uint8Array|undefined} data
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
   * @returns {Uint8Array|undefined}
   */
  getData() {
    return this.data;
  }
}

export default ErrorResult;
