import {AbstractError} from "../../errors/AbstractError.js";

export class ConfigFileNotFoundError extends AbstractError {
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
