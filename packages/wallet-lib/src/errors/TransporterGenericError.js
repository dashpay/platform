const WalletLibError = require('./WalletLibError');

class TransporterGenericError extends WalletLibError {
  constructor(act, reason) {
    super(`Unable to ${act}, reason: ${reason}`);
  }
}
module.exports = TransporterGenericError;
