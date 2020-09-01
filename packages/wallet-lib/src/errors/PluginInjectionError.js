const WalletLibError = require('./WalletLibError');

class PluginInjectionError extends WalletLibError {
  constructor(error) {
    super(`Failed to perform standard injections with reason: ${error.message}`);

    this.error = error;
  }

  getError() {
    return this.error;
  }
}

module.exports = PluginInjectionError;
