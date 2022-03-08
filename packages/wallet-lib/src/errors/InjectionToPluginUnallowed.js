const WalletLibError = require('./WalletLibError');

class InjectionToPluginUnallowed extends WalletLibError {
  constructor(currentPluginName, injectingPluginName) {
    super(`Injection of plugin : ${injectingPluginName} into ${currentPluginName} not allowed`);
  }
}

module.exports = InjectionToPluginUnallowed;
