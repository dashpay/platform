import { AbstractError } from '../../errors/AbstractError.js';

export class HomeDirDoesNotExistError extends AbstractError {
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
