const WalletLibError = require('./WalletLibError');

class InjectionToPluginUnallowed extends WalletLibError {
  constructor(pluginName) {
    super(`Injection of plugin : ${pluginName} Unallowed`);
  }
}

module.exports = InjectionToPluginUnallowed;
