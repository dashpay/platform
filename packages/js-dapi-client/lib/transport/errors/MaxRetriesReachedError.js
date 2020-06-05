const DAPIClientError = require('../../errors/DAPIClientError');

class MaxRetriesReachedError extends DAPIClientError {
  /**
   * @param {Error} error
   */
  constructor(error) {
    super(`Max retries reached: ${error.message}`);

    this.error = error;
  }

  /**
   * @returns {Error}
   */
  getError() {
    return this.error;
  }
}

module.exports = MaxRetriesReachedError;
