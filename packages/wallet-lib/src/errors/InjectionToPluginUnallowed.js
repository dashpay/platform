const WalletLibError = require('./WalletLibError');

class InjectionToPluginUnallowed extends WalletLibError {
  constructor(pluginName) {
    const getErrorMessageOf = () => `Injection of plugin : ${pluginName} Unallowed `;

    super(getErrorMessageOf((pluginName)));
  }
}

module.exports = InjectionToPluginUnallowed;
