const WalletLibError = require('./WalletLibError');

class InstantLockTimeoutError extends WalletLibError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super(`InstantLock waiting period for transaction ${transactionHash} timed out`);
  }
}

module.exports = InstantLockTimeoutError;
