import WalletLibError from './WalletLibError.js';

class PluginFailedOnStart extends WalletLibError {
  constructor(pluginType, pluginName, error) {
    super(`Plugin ${pluginName} of type ${pluginType} onStart failed: ${error.message}`);

    this.error = error;
  }

  getError() {
    return this.error;
  }
}

export default PluginFailedOnStart;