const WalletLibError = require('./WalletLibError');

class UnknownPlugin extends WalletLibError {
  constructor(pluginName) {
    const getErrorMessageOf = () => `Unknown Plugin : ${pluginName}.`;
    super(getErrorMessageOf());
  }
}

module.exports = UnknownPlugin;
