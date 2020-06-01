const WalletLibError = require('./WalletLibError');

class WorkerFailedOnExecute extends WalletLibError {
  constructor(pluginName, reason = 'unknown') {
    super(`Worker ${pluginName} failed onExecute. Reason: ${reason}`);
  }
}

module.exports = WorkerFailedOnExecute;
