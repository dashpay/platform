const WalletLibError = require('./WalletLibError');

class InvalidStrategy extends WalletLibError {
  constructor(arg) {
    const type = arg.constructor.name;
    super(`Unable to import strategy. Expected 'str' or 'fn' got ${type}`);
  }
}
module.exports = InvalidStrategy;
