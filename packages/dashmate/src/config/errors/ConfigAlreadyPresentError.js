const AbstractError = require('../../errors/AbstractError');

class ConfigAlreadyPresentError extends AbstractError {
  /**
   * @param {string} configName
   */
  constructor(configName) {
    super(`Config with name '${configName}' already present`);

    this.configName = configName;
  }

  /**
   * @returns {string}
   */
  getConfigName() {
    return this.configName;
  }
}

module.exports = ConfigAlreadyPresentError;
