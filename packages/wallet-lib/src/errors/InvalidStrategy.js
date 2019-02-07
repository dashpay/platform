const WalletLibError = require('./WalletLibError');

class InvalidStrategy extends WalletLibError {
  constructor(arg) {
    const type = arg.constructor.name;
    const getErrorMessageOf = () => `Unable to import strategy. Expected 'str' or 'fn' got ${type}`;
    super(getErrorMessageOf(arg));
  }
}
module.exports = InvalidStrategy;
