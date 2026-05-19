import WalletLibError from './WalletLibError.js';

class InvalidStorageAdapter extends WalletLibError {
  constructor(reason) {
    super(`Invalid Storage Adapter : ${reason}`);
  }
}
export default InvalidStorageAdapter;