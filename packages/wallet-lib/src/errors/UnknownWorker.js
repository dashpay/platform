import WalletLibError from './WalletLibError.js';

class UnknownWorker extends WalletLibError {
  constructor(workerName) {
    const getErrorMessageOf = () => `Unknown Worker : ${workerName}.`;
    super(getErrorMessageOf());
  }
}

export default UnknownWorker;