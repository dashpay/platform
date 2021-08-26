const DPPError = require('./DPPError');

class MissingOptionError extends DPPError {
  /**
   * @param {string} optionName
   * @param {string} message
   */
  constructor(optionName, message) {
    super(message);

    this.optionName = optionName;
  }

  /**
   * @return {string}
   */
  getOptionName() {
    return this.optionName;
  }
}

module.exports = MissingOptionError;
