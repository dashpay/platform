const DriveError = require('../../../errors/DriveError');

class NetworkProtocolVersionIsNotSetError extends DriveError {
  constructor() {
    super('Network protocol version is not set');
  }
}

module.exports = NetworkProtocolVersionIsNotSetError;
