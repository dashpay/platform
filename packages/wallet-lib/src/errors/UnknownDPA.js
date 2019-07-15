const WalletLibError = require('./WalletLibError');

class UnknownDPA extends WalletLibError {
  constructor(dpaName) {
    const getErrorMessageOf = () => `Unknown DPA : ${dpaName}.`;
    super(getErrorMessageOf());
  }
}

module.exports = UnknownDPA;
