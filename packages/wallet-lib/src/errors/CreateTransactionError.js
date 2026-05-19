import WalletLibError from './WalletLibError.js';
import CoinSelectionUnsufficientUTXOS from './CoinSelectionUnsufficientUTXOS.js';

class CreateTransactionError extends WalletLibError {
  constructor(e) {
    if (e instanceof CoinSelectionUnsufficientUTXOS) {
      super('Unsufficient funds to cover the output');
    } else {
      super(e);
    }
  }
}
export default CreateTransactionError;