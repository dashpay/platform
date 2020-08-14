const AbstractError = require('../../errors/AbstractError');

class InvalidConfigFileFormatError extends AbstractError {
  /**
   * @param {string} configFilePath
   * @param {Error} error
   */
  constructor(configFilePath, error) {
    super(`Invalid '${configFilePath}' config format: ${error.message}`);

    this.error = error;
    this.configFilePath = configFilePath;
  }

  /**
   * @returns {Error}
   */
  getError() {
    return this.error;
  }

  /**
   * @returns {string}
   */
  getConfigFilePath() {
    return this.configFilePath;
  }
}

module.exports = InvalidConfigFileFormatError;
