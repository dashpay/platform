const AbstractError = require('../../errors/AbstractError');

class InvalidOptionsError extends AbstractError {
  /**
   * @param {Object} options
   * @param {ErrorObject[]} errors
   * @param {string} message
   */
  constructor(options, errors, message) {
    super(message);

    this.options = options;
    this.errors = errors;
  }

  /**
   * @returns {Object}
   */
  getOptions() {
    return this.options;
  }

  /**
   * @returns {ErrorObject[]}
   */
  getErrors() {
    return this.errors;
  }
}

module.exports = InvalidOptionsError;
