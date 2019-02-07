const WalletLibError = require('./WalletLibError');

class InjectionToPluginUnallowed extends WalletLibError {}

module.exports = InjectionToPluginUnallowed;
