import WalletLibError from './WalletLibError.js';

class InjectionErrorCannotInject extends WalletLibError {
  constructor(pluginName, reason) {
    const getErrorMessageOf = () => `Injection of plugin : ${pluginName} impossible.
     Reason : ${reason}`;

    super(getErrorMessageOf());
  }
}

export default InjectionErrorCannotInject;