import WalletLibError from './WalletLibError.js';

class InstantLockTimeoutError extends WalletLibError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super(`InstantLock waiting period for transaction ${transactionHash} timed out`);
  }
}

export default InstantLockTimeoutError;