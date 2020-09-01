const WalletLibError = require('./WalletLibError');

class WorkerFailedOnExecute extends WalletLibError {
  /**
   * @param {string} pluginName
   * @param {Error} error
   */
  constructor(pluginName, error) {
    super(`Worker ${pluginName} failed onExecute: ${error.message}`);

    this.error = error;
  }

  /**
   * @returns {Error}
   */
  getError() {
    return this.error;
  }
}

module.exports = WorkerFailedOnExecute;
