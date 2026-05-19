import WalletLibError from './WalletLibError.js';

class InjectionToPluginUnallowed extends WalletLibError {
  constructor(currentPluginName, injectingPluginName) {
    super(`Injection of plugin : ${injectingPluginName} into ${currentPluginName} not allowed`);
  }
}

export default InjectionToPluginUnallowed;