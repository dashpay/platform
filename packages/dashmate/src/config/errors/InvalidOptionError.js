const AbstractError = require('../../errors/AbstractError');

class InvalidOptionPathError extends AbstractError {
  /**
   * @param {string} path
   * @param {*} value
   * @param {ErrorObject[]} errors
   * @param {string} message
   */
  constructor(path, value, errors, message) {
    super(message);

    this.path = path;
    this.value = value;
    this.error = errors;
  }

  /**
   * @returns {string}
   */
  getPath() {
    return this.path;
  }

  /**
   * @returns {*}
   */
  getValue() {
    return this.value;
  }

  /**
   * @returns {ErrorObject[]}
   */
  getErrors() {
    return this.errors;
  }
}

module.exports = InvalidOptionPathError;
