const AbstractError = require('../../errors/AbstractError');

class OptionIsNotSetError extends AbstractError {
  /**
   * @param {Config} config
   * @param {string} path
   */
  constructor(config, path) {
    super(`${path} option is not set in ${config.getName()} config`);

    this.config = config;
    this.path = path;
  }

  /**
   * @returns {string}
   */
  getPath() {
    return this.path;
  }

  /**
   * @returns {Config}
   */
  getConfig() {
    return this.config;
  }

  /**
   * @returns {ErrorObject[]}
   */
  getErrors() {
    return this.errors;
  }
}

module.exports = OptionIsNotSetError;
