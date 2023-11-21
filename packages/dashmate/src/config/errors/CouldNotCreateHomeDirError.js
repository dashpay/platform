import AbstractError from '../../errors/AbstractError.js';

export default class CouldNotCreateHomeDirError extends AbstractError {
  /**
   * @param {string} homeDirPath
   * @param {Error} cause
   */
  constructor(homeDirPath, cause) {
    super(`Could not create home dir at '${homeDirPath}': ${cause}`);

    this.cause = cause;

    this.homeDirPath = homeDirPath;
  }

  /**
   * @returns {string}
   */
  getHomeDirPath() {
    return this.homeDirPath;
  }
}
