const WalletLibError = require('./WalletLibError');

class InvalidStorageAdapter extends WalletLibError {
  constructor(reason) {
    super(`Invalid Storage Adapter : ${reason}`);
  }
}
module.exports = InvalidStorageAdapter;
