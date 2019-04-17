const WalletLibError = require('./WalletLibError');

class UnknownStrategy extends WalletLibError {
  constructor(strategyName) {
    const getErrorMessageOf = () => `Unknown Strategy : ${strategyName}.`;
    super(getErrorMessageOf());
  }
}

module.exports = UnknownStrategy;
