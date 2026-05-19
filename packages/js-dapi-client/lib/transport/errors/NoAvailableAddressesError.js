import DAPIClientError from '../../errors/DAPIClientError.js';

class NoAvailableAddressesError extends DAPIClientError {
  constructor() {
    super('No available addresses');
  }
}

export default NoAvailableAddressesError;
