import {AbstractError} from "../../errors/AbstractError.js";

export class ConfigAlreadyPresentError extends AbstractError {
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
