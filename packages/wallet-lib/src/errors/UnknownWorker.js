const WalletLibError = require('./WalletLibError');

class UnknownWorker extends WalletLibError {
  constructor(workerName) {
    const getErrorMessageOf = () => `Unknown Worker : ${workerName}.`;
    super(getErrorMessageOf());
  }
}

module.exports = UnknownWorker;
