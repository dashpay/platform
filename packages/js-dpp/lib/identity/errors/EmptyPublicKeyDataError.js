const DPPError = require('../../errors/DPPError');

class EmptyPublicKeyDataError extends DPPError {
  constructor() {
    super('Public key data is not set');
  }
}

module.exports = EmptyPublicKeyDataError;
