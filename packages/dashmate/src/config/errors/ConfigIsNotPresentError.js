import AbstractError from '../../errors/AbstractError.js';

export default class ConfigIsNotPresentError extends AbstractError {
  /**
   * @param {string} configName
   */
  constructor(configName) {
    super(`Config with name '${configName}' is not present`);

    this.configName = configName;
  }

  /**
   * @returns {string}
   */
  getConfigName() {
    return this.configName;
  }
}
