import WalletLibError from './WalletLibError.js';

class MempoolPropagationTimeoutError extends WalletLibError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super(`Mempool propagation waiting period for transaction ${transactionHash} timed out`);
  }
}

export default MempoolPropagationTimeoutError;