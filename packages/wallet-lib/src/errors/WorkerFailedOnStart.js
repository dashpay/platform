const WalletLibError = require('./WalletLibError');

class WorkerFailedOnStart extends WalletLibError {
  constructor(pluginName, reason = 'unknown') {
    const message = `Worker ${pluginName} failed onStart. Reason: ${reason}`;
    super(message);
  }
}

module.exports = WorkerFailedOnStart;
