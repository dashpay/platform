import DAPIClientError from '../../../../errors/DAPIClientError.js';

class InvalidResponseError extends DAPIClientError {
  constructor(message) {
    super(`Invalid response: ${message}`);
  }
}

export default InvalidResponseError;
