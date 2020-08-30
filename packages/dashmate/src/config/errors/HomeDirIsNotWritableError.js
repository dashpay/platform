const AbstractError = require('../../errors/AbstractError');

class HomeDirIsNotWritableError extends AbstractError {
  /**
   * @param {string} homeDirPath
   */
  constructor(homeDirPath) {
    super(`Home dir '${homeDirPath}' is not writeable`);

    this.homeDirPath = homeDirPath;
  }

  /**
   * @returns {string}
   */
  getHomeDirPath() {
    return this.homeDirPath;
  }
}

module.exports = HomeDirIsNotWritableError;
