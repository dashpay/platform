import AbstractError from '../../errors/AbstractError.js';

export default class HomeDirIsNotWritableError extends AbstractError {
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
