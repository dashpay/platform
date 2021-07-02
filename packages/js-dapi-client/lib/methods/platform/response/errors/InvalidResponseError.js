const DAPIClientError = require('../../../../errors/DAPIClientError');

class InvalidResponseError extends DAPIClientError {
  constructor(message) {
    super(`Invalid response: ${message}`);
  }
}

module.exports = InvalidResponseError;
