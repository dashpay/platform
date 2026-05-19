import WalletLibError from './WalletLibError.js';

class BlockHeaderNotInStore extends WalletLibError {
  constructor(identifier) {
    super(`Blockheader is not in store: ${identifier}`);
  }
}
export default BlockHeaderNotInStore;