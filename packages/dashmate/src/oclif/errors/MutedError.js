const AbstractError = require('../../errors/AbstractError');

class MutedError extends AbstractError {
  /**
   * @param {Error} error
   */
  constructor(error) {
    super('SIGINT');

    this.error = error;
  }

  /**
   * Get thrown error
   * @return {Error}
   */
  getError() {
    return this.error;
  }
}

module.exports = MutedError;
