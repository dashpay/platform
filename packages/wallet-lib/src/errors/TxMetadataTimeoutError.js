const WalletLibError = require('./WalletLibError');

class TxMetadataTimeoutError extends WalletLibError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super(`Metadata waiting period for transaction ${transactionHash} timed out`);
  }
}

module.exports = TxMetadataTimeoutError;
