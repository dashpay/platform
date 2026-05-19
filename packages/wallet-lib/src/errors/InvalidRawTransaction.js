import WalletLibError from './WalletLibError.js';

class InvalidTransaction extends WalletLibError {
  constructor() {
    super('A valid transaction object or it\'s hex representation is required');
  }
}

export default InvalidTransaction;