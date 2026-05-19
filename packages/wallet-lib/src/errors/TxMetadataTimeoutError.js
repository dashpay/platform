import WalletLibError from './WalletLibError.js';

class TxMetadataTimeoutError extends WalletLibError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super(`Metadata waiting period for transaction ${transactionHash} timed out`);
  }
}

export default TxMetadataTimeoutError;