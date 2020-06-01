const WalletLibError = require('./WalletLibError');

class WorkerFailedOnStart extends WalletLibError {
  constructor(pluginName, reason = 'unknown') {
    super(`Worker ${pluginName} failed onStart. Reason: ${reason}`);
  }
}

module.exports = WorkerFailedOnStart;
