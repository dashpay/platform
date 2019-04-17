const WalletLibError = require('./WalletLibError');

class UnknownDAP extends WalletLibError {
  constructor(dapName) {
    const getErrorMessageOf = () => `Unknown DAP : ${dapName}.`;
    super(getErrorMessageOf());
  }
}

module.exports = UnknownDAP;
