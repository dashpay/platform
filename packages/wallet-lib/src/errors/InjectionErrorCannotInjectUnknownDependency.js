const WalletLibError = require('./WalletLibError');

class InjectionErrorCannotInjectUnknownDependency extends WalletLibError {
  constructor(pluginName, dependencyName) {
    const getErrorMessageOf = () => `Injection of plugin : ${pluginName} impossible.
     Unknown Dependency ${dependencyName}`;

    super(getErrorMessageOf());
  }
}

module.exports = InjectionErrorCannotInjectUnknownDependency;
