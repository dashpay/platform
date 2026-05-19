import WalletLibError from './WalletLibError.js';

class InjectionErrorCannotInjectUnknownDependency extends WalletLibError {
  constructor(pluginName, dependencyName) {
    const getErrorMessageOf = () => `Injection of plugin : ${pluginName} impossible.
     Unknown Dependency ${dependencyName}`;

    super(getErrorMessageOf());
  }
}

export default InjectionErrorCannotInjectUnknownDependency;