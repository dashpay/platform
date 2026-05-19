import WalletLibError from './WalletLibError.js';

class TransactionNotInStore extends WalletLibError {
  constructor(txid) {
    super(`Transaction is not in store: ${txid}`);
  }
}

export default TransactionNotInStore;