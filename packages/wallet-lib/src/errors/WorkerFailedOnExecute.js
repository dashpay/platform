const WalletLibError = require('./WalletLibError');

class WorkerFailedOnExecute extends WalletLibError {
  constructor(pluginName, reason = 'unknown') {
    const message = `Worker ${pluginName} failed onExecute. Reason: ${reason}`;
    super(message);
  }
}

module.exports = WorkerFailedOnExecute;
