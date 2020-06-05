const DAPIClientError = require('../../errors/DAPIClientError');

class DAPIAddressHostMissingError extends DAPIClientError {
  constructor() {
    super('Host is required for DAPI address');
  }
}

module.exports = DAPIAddressHostMissingError;
