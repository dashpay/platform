const DAPIClientError = require('../../errors/DAPIClientError');

class NoAvailableAddressesForRetry extends DAPIClientError {
  /**
   * @param {Error} error
   */
  constructor(error) {
    super(`No available addresses for retry: ${error.message}`);

    this.error = error;
  }

  /**
   * @returns {Error}
   */
  getError() {
    return this.error;
  }
}

module.exports = NoAvailableAddressesForRetry;
