const WalletLibError = require('./WalletLibError');

class InvalidUTXO extends WalletLibError {
  constructor() {
    const message = 'Invalid UnspentOutput provided.';
    super(message);
  }
}
module.exports = InvalidUTXO;
