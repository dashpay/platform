import WalletLibError from './WalletLibError.js';

class TransporterGenericError extends WalletLibError {
  constructor(act, reason) {
    super(`Unable to ${act}, reason: ${reason}`);
  }
}
export default TransporterGenericError;