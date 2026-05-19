import WalletLibError from './WalletLibError.js';

class WorkerFailedOnStart extends WalletLibError {
  /**
   * @param {string} pluginName
   * @param {Error} error
   */
  constructor(pluginName, error) {
    super(`Worker ${pluginName} failed onStart: ${error.message}`);

    this.error = error;
  }

  /**
   * @returns {Error}
   */
  getError() {
    return this.error;
  }
}

export default WorkerFailedOnStart;