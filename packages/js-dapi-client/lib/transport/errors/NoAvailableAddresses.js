const DAPIClientError = require('../../errors/DAPIClientError');

class NoAvailableAddresses extends DAPIClientError {
  constructor() {
    super('No available addresses');
  }
}

module.exports = NoAvailableAddresses;
