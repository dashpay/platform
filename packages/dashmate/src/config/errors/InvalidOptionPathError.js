import { AbstractError } from '../../errors/AbstractError.js';

export class InvalidOptionPathError extends AbstractError {
  /**
   * @param {string} path
   */
  constructor(path) {
    super(`There is no option with '${path}' path`);

    this.path = path;
  }

  /**
   * @returns {string}
   */
  getPath() {
    return this.path;
  }
}
