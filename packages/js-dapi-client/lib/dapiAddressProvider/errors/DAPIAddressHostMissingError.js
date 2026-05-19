import DAPIClientError from '../../errors/DAPIClientError.js';

class DAPIAddressHostMissingError extends DAPIClientError {
  constructor() {
    super('Host is required for DAPI address');
  }
}

export default DAPIAddressHostMissingError;
