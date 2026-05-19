import WalletLibError from './WalletLibError.js';

class UnknownPlugin extends WalletLibError {
  constructor(pluginName) {
    const getErrorMessageOf = () => `Unknown Plugin : ${pluginName}.`;
    super(getErrorMessageOf());
  }
}

export default UnknownPlugin;