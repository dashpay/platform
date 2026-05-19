import WalletLibError from './WalletLibError.js';

class StorageUnableToAddTransaction extends WalletLibError {
  constructor(tx) {
    const getErrorMessageOf = (_tx) => `Unable to add transaction : ${JSON.stringify(_tx)}`;
    super(getErrorMessageOf(tx));
  }
}
export default StorageUnableToAddTransaction;