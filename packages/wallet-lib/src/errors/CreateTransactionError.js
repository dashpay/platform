const WalletLibError = require('./WalletLibError');
const CoinSelectionUnsufficientUTXOS = require('./CoinSelectionUnsufficientUTXOS');

class CreateTransactionError extends WalletLibError {
  constructor(e) {
    if (e instanceof CoinSelectionUnsufficientUTXOS) {
      super('Unsufficient funds to cover the output');
    } else {
      super(e);
    }
  }
}
module.exports = CreateTransactionError;
