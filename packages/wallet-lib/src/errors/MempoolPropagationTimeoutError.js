const WalletLibError = require('./WalletLibError');

class MempoolPropagationTimeoutError extends WalletLibError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super(`Mempool propagation waiting period for transaction ${transactionHash} timed out`);
  }
}

module.exports = MempoolPropagationTimeoutError;
