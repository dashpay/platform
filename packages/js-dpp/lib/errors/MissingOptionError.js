class MissingOptionError extends Error {
  /**
   * @param {string} optionName
   * @param {string} message
   */
  constructor(optionName, message) {
    super();

    this.name = this.constructor.name;
    this.optionName = optionName;
    this.message = message;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * @return {string}
   */
  getOptionName() {
    return this.optionName;
  }
}

module.exports = MissingOptionError;
