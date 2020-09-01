const WalletLibError = require('./WalletLibError');

class PluginFailedOnStart extends WalletLibError {
  constructor(pluginType, pluginName, error) {
    super(`Plugin ${pluginName} of type ${pluginType} onStart failed: ${error.message}`);

    this.error = error;
  }

  getError() {
    return this.error;
  }
}

module.exports = PluginFailedOnStart;
