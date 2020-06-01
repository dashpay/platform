const WalletLibError = require('./WalletLibError');

class PluginFailedOnStart extends WalletLibError {
  constructor(pluginType, pluginName, reason = 'unknown') {
    super(`Plugin ${pluginName} of type ${pluginType} onStart failed. Reason: ${reason}`);
  }
}

module.exports = PluginFailedOnStart;
