import WalletLibError from './WalletLibError.js';

class TransactionMetadataNotInStore extends WalletLibError {
  constructor(txid) {
    super(`Transaction metadata is not in store: ${txid}`);
  }
}

export default TransactionMetadataNotInStore;