import WalletLibError from './WalletLibError.js';

class InvalidDashcoreTransaction extends WalletLibError {
  constructor(tx, reason = 'A Dashcore Transaction object or valid rawTransaction is required') {
    super(`${reason}: ${tx.toString()}`);
  }
}

export default InvalidDashcoreTransaction;