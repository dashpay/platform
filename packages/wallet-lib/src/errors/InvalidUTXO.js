import WalletLibError from './WalletLibError.js';

class InvalidUTXO extends WalletLibError {
  constructor() {
    const message = 'Invalid UnspentOutput provided.';
    super(message);
  }
}
export default InvalidUTXO;