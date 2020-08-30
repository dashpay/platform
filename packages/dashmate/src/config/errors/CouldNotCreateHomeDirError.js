const AbstractError = require('../../errors/AbstractError');

class CouldNotCreateHomeDirError extends AbstractError {
  /**
   * @param {string} homeDirPath
   */
  constructor(homeDirPath) {
    super(`Could not create home dir at '${homeDirPath}'`);

    this.homeDirPath = homeDirPath;
  }

  /**
   * @returns {string}
   */
  getHomeDirPath() {
    return this.homeDirPath;
  }
}

module.exports = CouldNotCreateHomeDirError;
