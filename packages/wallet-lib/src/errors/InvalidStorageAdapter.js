const WalletLibError = require('./WalletLibError');

class InvalidStorageAdapter extends WalletLibError {
  constructor(reason) {
    const getErrorMessageOf = () => `Invalid Storage Adapter : ${reason}`;
    super(getErrorMessageOf((reason)));
  }
}
module.exports = InvalidStorageAdapter;
