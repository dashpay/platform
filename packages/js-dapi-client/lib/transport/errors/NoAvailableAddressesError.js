const DAPIClientError = require('../../errors/DAPIClientError');

class NoAvailableAddressesError extends DAPIClientError {
  constructor() {
    super('No available addresses');
  }
}

module.exports = NoAvailableAddressesError;
