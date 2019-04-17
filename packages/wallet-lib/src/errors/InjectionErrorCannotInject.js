const WalletLibError = require('./WalletLibError');

class InjectionErrorCannotInject extends WalletLibError {
  constructor(pluginName, reason) {
    const getErrorMessageOf = () => `Injection of plugin : ${pluginName} impossible.
     Reason : ${reason}`;

    super(getErrorMessageOf());
  }
}

module.exports = InjectionErrorCannotInject;
