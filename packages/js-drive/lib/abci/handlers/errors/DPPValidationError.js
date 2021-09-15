class DPPValidationError extends Error {
  /**
   *
   * @param {string} message
   * @param {AbstractConsenusError[]} errors
   */
  constructor(message, errors) {
    super(message);

    this.errors = errors;
  }

  /**
   * Get error code
   *
   * @returns {number}
   */
  getCode() {
    return this.errors[0].getCode();
  }

  /**
   * Get error info
   *
   * @returns {Array}
   */
  getInfo() {
    return this.errors[0].getConstructorArguments();
  }
}

module.exports = DPPValidationError;
