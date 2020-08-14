const AbstractError = require('../../errors/AbstractError');

class InvalidOptionPathError extends AbstractError {
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

module.exports = InvalidOptionPathError;
