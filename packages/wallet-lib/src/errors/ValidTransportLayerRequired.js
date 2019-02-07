const WalletLibError = require('./WalletLibError');

class ValidTransportLayerRequired extends WalletLibError {
  constructor(method) {
    super(`A transport layer is needed to perform a ${method}`);
  }
}
module.exports = ValidTransportLayerRequired;
