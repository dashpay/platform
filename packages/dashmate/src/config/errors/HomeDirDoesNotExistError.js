const AbstractError = require('../../errors/AbstractError');

class HomeDirDoesNotExistError extends AbstractError {
  /**
   * @param {string} homeDirPath
   */
  constructor(homeDirPath) {
    super(`Home dir '${homeDirPath}' does not exist`);

    this.homeDirPath = homeDirPath;
  }

  /**
   * @returns {string}
   */
  getHomeDirPath() {
    return this.homeDirPath;
  }
}

module.exports = HomeDirDoesNotExistError;
