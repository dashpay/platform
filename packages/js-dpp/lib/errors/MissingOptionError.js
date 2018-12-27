class MissingOptionError extends Error {
  /**
   * @param {string} optionName
   */
  constructor(optionName) {
    super();

    this.name = this.constructor.name;
    this.optionName = optionName;
    this.message = `${optionName} is not defined`;

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
