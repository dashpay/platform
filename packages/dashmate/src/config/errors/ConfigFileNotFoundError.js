const AbstractError = require('../../errors/AbstractError');

class ConfigFileNotFoundError extends AbstractError {
  /**
   * @param {string} configFilePath
   */
  constructor(configFilePath) {
    super(`Config file '${configFilePath}' not found`);

    this.configFilePath = configFilePath;
  }

  /**
   * @returns {string}
   */
  getConfigFilePath() {
    return this.configFilePath;
  }
}

module.exports = ConfigFileNotFoundError;
